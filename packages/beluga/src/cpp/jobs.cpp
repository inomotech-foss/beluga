#include <cstring>
#include "jobs.h"

extern "C" void on_subscribe_completed_jobs(const void *, int32_t);
extern "C" void on_publish_completed_jobs(const void *, const void *, int32_t);
extern "C" void on_next_job_execution_changed_events(const void *, JobInfo *, crt::DateTime *, int32_t);
extern "C" void on_start_next_pending_job_execution_accepted(const void *, char *, JobInfo *, int32_t);
extern "C" void on_start_next_pending_job_execution_rejected(const void *, Rejected, int32_t);
extern "C" void on_get_pending_job_executions_accepted(const void *, JobsSummary, int32_t);
extern "C" void on_get_pending_job_executions_rejected(const void *, Rejected, int32_t);
extern "C" void on_job_executions_changed_events(const void *, int32_t);

std::unique_ptr<JobInfo> get_job_info(jobs::JobExecutionData *data);
jobs::OnSubscribeToGetPendingJobExecutionsAcceptedResponse get_pending_job_executions(const void *interface);
jobs::OnSubscribeToJobExecutionsChangedEventsResponse job_executions_changed_events(const void *interface);
jobs::OnSubscribeToStartNextPendingJobExecutionAcceptedResponse start_next_pending_job_execution(const void *interface);
jobs::OnSubscribeToNextJobExecutionChangedEventsResponse next_job_execution_changed_events(const void *interface);

InternalJobsClient::InternalJobsClient(
    std::shared_ptr<jobs::IotJobsClient> client,
    const void *interface, const char *thing_name) : client(std::move(client)), interface(interface), thing_name(AwsString(thing_name))
{
}

std::shared_ptr<jobs::IotJobsClient> InternalJobsClient::internal_client()
{
    return this->client;
}

const void *InternalJobsClient::get_interface() const
{
    return this->interface;
}

AwsString InternalJobsClient::get_name() const
{
    return this->thing_name;
}

JobInfo::JobInfo() : job_id(nullptr), job_document(Buffer()),
                     status(nullptr), version_number(nullptr),
                     queue_at(nullptr), thing_name(nullptr),
                     execution_number(nullptr), last_updated_at(nullptr),
                     started_at(nullptr)
{
}

Rejected::Rejected() : timestamp(nullptr), code(nullptr),
                       message(nullptr), client_token(nullptr)
{
}

JobExecutionSummary::JobExecutionSummary() : job_id(nullptr), version_number(nullptr),
                                             execution_number(nullptr), started_at(nullptr),
                                             queued_at(nullptr), last_updated_at(nullptr)
{
}

JobExecutionSummary::JobExecutionSummary(
    const char *job_id, const int32_t *version_number,
    const int64_t *execution_number, const crt::DateTime *started_at,
    const crt::DateTime *queued_at, const crt::DateTime *last_updated_at) : job_id(job_id), version_number(version_number),
                                                                            execution_number(execution_number), started_at(started_at),
                                                                            queued_at(queued_at), last_updated_at(last_updated_at)
{
}

JobsSummary::JobsSummary() : queued_jobs(nullptr), progres_jobs(nullptr),
                             queued_size(0), progres_size(0)
{
}

JobsSummary::JobsSummary(
    JobExecutionSummary *queued_jobs, JobExecutionSummary *progres_jobs,
    size_t queued_size, size_t progres_size) : queued_jobs(queued_jobs), progres_jobs(progres_jobs),
                                               queued_size(queued_size), progres_size(progres_size)
{
}

InternalJobsClient *internal_jobs_client(
    InternalMqttClient *mqtt_client, const void *interface, QOS qos, const char *thing_name)
{
    auto jobs_client = std::make_shared<jobs::IotJobsClient>(mqtt_client->get_connection());

    auto req = jobs::GetPendingJobExecutionsSubscriptionRequest();
    req.ThingName = crt::Optional<AwsString>(AwsString(thing_name));

    if (!jobs_client->SubscribeToGetPendingJobExecutionsAccepted(req, qos, get_pending_job_executions(interface),
                                                                 subscribe_completed(interface, on_subscribe_completed_jobs)))
    {
        return nullptr;
    }

    if (!jobs_client->SubscribeToGetPendingJobExecutionsRejected(req, qos, rejected(interface, on_get_pending_job_executions_rejected),
                                                                 subscribe_completed(interface, on_subscribe_completed_jobs)))
    {
        return nullptr;
    }

    auto executions_changed_req = jobs::JobExecutionsChangedSubscriptionRequest();
    executions_changed_req.ThingName = crt::Optional<AwsString>(AwsString(thing_name));

    if (!jobs_client->SubscribeToJobExecutionsChangedEvents(executions_changed_req, qos, job_executions_changed_events(interface),
                                                            subscribe_completed(interface, on_subscribe_completed_jobs)))
    {
        return nullptr;
    }

    auto execution_changed_req = jobs::NextJobExecutionChangedSubscriptionRequest();
    execution_changed_req.ThingName = crt::Optional<AwsString>(AwsString(thing_name));
    if (!jobs_client->SubscribeToNextJobExecutionChangedEvents(execution_changed_req, qos, next_job_execution_changed_events(interface),
                                                               subscribe_completed(interface, on_subscribe_completed_jobs)))
    {
        return nullptr;
    }

    auto next_req = jobs::StartNextPendingJobExecutionSubscriptionRequest();
    next_req.ThingName = crt::Optional<AwsString>(AwsString(thing_name));

    if (!jobs_client->SubscribeToStartNextPendingJobExecutionAccepted(next_req, qos, start_next_pending_job_execution(interface),
                                                                      subscribe_completed(interface, on_subscribe_completed_jobs)))
    {
        return nullptr;
    }

    if (!jobs_client->SubscribeToStartNextPendingJobExecutionRejected(next_req, qos, rejected(interface, on_start_next_pending_job_execution_rejected),
                                                                      subscribe_completed(interface, on_subscribe_completed_jobs)))
    {
        return nullptr;
    }

    return new InternalJobsClient(std::move(jobs_client), interface, thing_name);
}

bool publish_get_pending_executions(InternalJobsClient *client, QOS qos, const void *callback)
{
    auto req = jobs::GetPendingJobExecutionsRequest();
    req.ThingName = crt::Optional<AwsString>(client->get_name());
    return client->internal_client()->PublishGetPendingJobExecutions(
        req, qos, publish_complete(client->get_interface(), callback, on_publish_completed_jobs));
}

bool publish_start_next_pending_execution(InternalJobsClient *client, QOS qos, const void *callback, NextPendingRequest request)
{
    auto req = jobs::StartNextPendingJobExecutionRequest();
    req.ThingName = crt::Optional<AwsString>(client->get_name());

    if (request.step_timeout)
    {
        req.StepTimeoutInMinutes = crt::Optional<int64_t>(*request.step_timeout);
    }

    return client->internal_client()->PublishStartNextPendingJobExecution(
        req, qos, publish_complete(client->get_interface(), callback, on_publish_completed_jobs));
}

void drop_jobs_client(InternalJobsClient *client)
{
    delete client;
}

jobs::OnSubscribeComplete subscribe_completed(const void *interface, std::function<void(const void *, int32_t)> impl)
{
    return [=](int32_t io_err)
    {
        impl(interface, io_err);
    };
}

jobs::OnSubscribeComplete publish_complete(const void *interface, const void *callback, std::function<void(const void *, const void *, int32_t)> impl)
{
    return [=](int32_t io_err)
    {
        impl(interface, callback, io_err);
    };
}

std::function<void(Aws::Iotjobs::RejectedError *, int32_t ioErr)> rejected(const void *interface, std::function<void(const void *, Rejected, int32_t)> impl)
{
    return [=](jobs::RejectedError *rejected, int32_t io_err)
    {
        auto rejected_err = Rejected();

        if (rejected->ClientToken)
        {
            rejected_err.client_token = const_cast<char *>(rejected->ClientToken->c_str());
        }

        if (rejected->Code)
        {
            rejected_err.code = &*rejected->Code;
        }

        if (rejected->Message)
        {
            rejected_err.message = const_cast<char *>(rejected->Message->c_str());
        }

        if (rejected->Timestamp)
        {
            rejected_err.timestamp = &*rejected->Timestamp;
        }

        impl(interface, rejected_err, io_err);
    };
}

jobs::OnSubscribeToGetPendingJobExecutionsAcceptedResponse get_pending_job_executions(const void *interface)
{
    return [=](jobs::GetPendingJobExecutionsResponse *response, int32_t io_err)
    {
        uint32_t queued_size = response->QueuedJobs ? response->QueuedJobs->size() : 0;
        uint32_t progress_size = response->InProgressJobs ? response->InProgressJobs->size() : 0;

        auto queued_jobs = std::make_unique<JobExecutionSummary[]>(queued_size);
        auto progress_jobs = std::make_unique<JobExecutionSummary[]>(progress_size);

        if (response->QueuedJobs)
        {
            for (uint32_t i = 0; i < queued_size; ++i)
            {
                const auto summary = response->QueuedJobs->at(i);
                queued_jobs[i] = JobExecutionSummary(summary.JobId ? summary.JobId->c_str() : nullptr,
                                                     summary.VersionNumber ? &*summary.VersionNumber : nullptr,
                                                     summary.ExecutionNumber ? &*summary.ExecutionNumber : nullptr,
                                                     summary.StartedAt ? &*summary.StartedAt : nullptr,
                                                     summary.QueuedAt ? &*summary.QueuedAt : nullptr,
                                                     summary.LastUpdatedAt ? &*summary.LastUpdatedAt : nullptr);
            }
        }

        if (response->InProgressJobs)
        {
            for (uint32_t i = 0; i < progress_size; ++i)
            {
                const auto summary = response->InProgressJobs->at(i);
                progress_jobs[i] = JobExecutionSummary(summary.JobId ? summary.JobId->c_str() : nullptr,
                                                       summary.VersionNumber ? &*summary.VersionNumber : nullptr,
                                                       summary.ExecutionNumber ? &*summary.ExecutionNumber : nullptr,
                                                       summary.StartedAt ? &*summary.StartedAt : nullptr,
                                                       summary.QueuedAt ? &*summary.QueuedAt : nullptr,
                                                       summary.LastUpdatedAt ? &*summary.LastUpdatedAt : nullptr);
            }
        }

        on_get_pending_job_executions_accepted(interface, JobsSummary(queued_jobs.get(), progress_jobs.get(), queued_size, progress_size), io_err);
    };
}

jobs::OnSubscribeToStartNextPendingJobExecutionAcceptedResponse start_next_pending_job_execution(const void *interface)
{
    return [=](jobs::StartNextJobExecutionResponse *response, int32_t io_err)
    {
        auto info = std::make_unique<JobInfo>();
        char *client_token = nullptr;

        if (response->Execution)
        {
            info = get_job_info(&*response->Execution);
        }

        if (response->ClientToken)
        {
            client_token = const_cast<char *>(response->ClientToken->c_str());
        }

        on_start_next_pending_job_execution_accepted(interface, client_token, info.get(), io_err);
    };
}

jobs::OnSubscribeToJobExecutionsChangedEventsResponse job_executions_changed_events(const void *interface)
{
    return [=](jobs::JobExecutionsChangedEvent *, int32_t io_err)
    {
        on_job_executions_changed_events(interface, io_err);
    };
}

jobs::OnSubscribeToNextJobExecutionChangedEventsResponse next_job_execution_changed_events(const void *interface)
{
    return [=](jobs::NextJobExecutionChangedEvent *response, int32_t io_err)
    {
        auto info = std::make_unique<JobInfo>();
        auto date_time = std::unique_ptr<crt::DateTime>();

        if (response->Execution)
        {
            info = get_job_info(&*response->Execution);
        }

        if (response->Timestamp)
        {
            date_time = std::make_unique<crt::DateTime>(response->Timestamp->Millis());
        }

        on_next_job_execution_changed_events(interface, info.get(), date_time.get(), io_err);
    };
}

std::unique_ptr<JobInfo> get_job_info(jobs::JobExecutionData *data)
{
    auto info = std::make_unique<JobInfo>();

    if (data->JobId)
    {
        info->job_id = const_cast<char *>(data->JobId->c_str());
    }

    if (data->JobDocument)
    {
        AwsString json = data->JobDocument->View().WriteCompact();
        auto buff = Buffer::create(json.size());

        if (!buff.is_empty())
        {
            std::memcpy(buff.data, json.data(), json.size());
            info->job_document = std::move(buff);
        }
    }

    if (data->ExecutionNumber)
    {
        info->execution_number = &*data->ExecutionNumber;
    }

    if (data->Status)
    {
        info->status = &*data->Status;
    }

    if (data->ThingName)
    {
        info->thing_name = const_cast<char *>(data->ThingName->c_str());
    }

    if (data->QueuedAt)
    {
        info->queue_at = &*data->QueuedAt;
    }

    if (data->StartedAt)
    {
        info->started_at = &*data->StartedAt;
    }

    if (data->LastUpdatedAt)
    {
        info->last_updated_at = &*data->LastUpdatedAt;
    }

    return info;
}
