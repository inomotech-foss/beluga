#include <cstring>
#include "jobs.h"

extern "C" void on_describe_job_execution_accepted(const void *, char *, JobInfo *, int32_t);
extern "C" void on_describe_job_execution_rejected(const void *, Rejected, int32_t);
extern "C" void on_update_job_execution_accepted(const void *, char *, JobInfo *, int32_t);
extern "C" void on_update_job_execution_rejected(const void *, Rejected, int32_t);
extern "C" void on_subscribe_completed_job(const void *, int32_t);
extern "C" void on_publish_completed_job(const void *, const void *, int32_t);

jobs::OnSubscribeToDescribeJobExecutionAcceptedResponse describe_job_execution_accepted(const void *interface);
jobs::OnSubscribeToUpdateJobExecutionAcceptedResponse update_job_execution_accepted(const void *interface);

InternalJob::InternalJob(
    std::shared_ptr<jobs::IotJobsClient> client,
    const void *interface, const char *thing_name,
    const char *job_id) : client(std::move(client)), interface(interface),
                          thing_name(AwsString(thing_name)), job_id(AwsString(job_id))
{
}

const void *InternalJob::get_interface() const
{
    return this->interface;
}

std::shared_ptr<jobs::IotJobsClient> InternalJob::internal_client()
{
    return this->client;
}

AwsString InternalJob::get_name() const
{
    return this->thing_name;
}

AwsString InternalJob::get_job_id() const
{
    return this->job_id;
}

InternalJob *internal_job(
    InternalMqttClient *mqtt_client, const void *interface, QOS qos,
    const char *thing_name, const char *job_id)
{
    auto jobs_client = std::make_shared<jobs::IotJobsClient>(mqtt_client->get_connection());

    auto request = jobs::DescribeJobExecutionSubscriptionRequest();
    request.ThingName = crt::Optional<AwsString>(AwsString(thing_name));
    request.JobId = crt::Optional<AwsString>(AwsString(job_id));

    if (!jobs_client->SubscribeToDescribeJobExecutionAccepted(request, qos, describe_job_execution_accepted(interface),
                                                              subscribe_completed(interface, on_subscribe_completed_job)))
    {
        return nullptr;
    }

    if (!jobs_client->SubscribeToDescribeJobExecutionRejected(request, qos, rejected(interface, on_describe_job_execution_rejected),
                                                              subscribe_completed(interface, on_subscribe_completed_job)))
    {
        return nullptr;
    }

    auto update_req = jobs::UpdateJobExecutionSubscriptionRequest();
    update_req.ThingName = crt::Optional<AwsString>(AwsString(thing_name));
    update_req.JobId = crt::Optional<AwsString>(AwsString(job_id));

    if (!jobs_client->SubscribeToUpdateJobExecutionAccepted(update_req, qos, update_job_execution_accepted(interface),
                                                            subscribe_completed(interface, on_subscribe_completed_job)))
    {
        return nullptr;
    }

    if (!jobs_client->SubscribeToUpdateJobExecutionRejected(update_req, qos, rejected(interface, on_update_job_execution_rejected),
                                                            subscribe_completed(interface, on_subscribe_completed_job)))
    {
        return nullptr;
    }

    return new InternalJob(std::move(jobs_client), interface, thing_name, job_id);
}

bool publish_describe_execution(InternalJob *job, QOS qos, const void *callback, DescribeExecutionRequest request)
{
    auto req = jobs::DescribeJobExecutionRequest();
    req.ThingName = crt::Optional<AwsString>(job->get_name());

    if (request.execution_number)
    {
        req.ExecutionNumber = crt::Optional<int64_t>(*request.execution_number);
    }

    if (request.include_document)
    {
        req.IncludeJobDocument = crt::Optional<bool>(*request.include_document);
    }

    if (request.job_id)
    {
        req.JobId = crt::Optional<AwsString>(AwsString(request.job_id));
    }

    return job->internal_client()->PublishDescribeJobExecution(
        req, qos, publish_complete(job->get_interface(), callback, on_publish_completed_job));
}

bool publish_update_execution(InternalJob *job, QOS qos, const void *callback, UpdateExecutionRequest request)
{
    auto req = jobs::UpdateJobExecutionRequest();
    req.ThingName = crt::Optional<AwsString>(job->get_name());

    if (request.execution_number)
    {
        req.ExecutionNumber = crt::Optional<int64_t>(*request.execution_number);
    }

    if (request.expected_version)
    {
        req.ExpectedVersion = crt::Optional<int32_t>(*request.expected_version);
    }

    if (request.include_document)
    {
        req.IncludeJobDocument = crt::Optional<bool>(*request.include_document);
    }

    if (request.include_execution_state)
    {
        req.IncludeJobExecutionState = crt::Optional<bool>(*request.include_execution_state);
    }

    if (request.job_id)
    {
        req.JobId = crt::Optional<AwsString>(AwsString(request.job_id));
    }

    if (request.status)
    {
        req.Status = crt::Optional<jobs::JobStatus>(*request.status);
    }

    if (request.step_timeout)
    {
        req.StepTimeoutInMinutes = crt::Optional<int64_t>(*request.step_timeout);
    }

    return job->internal_client()->PublishUpdateJobExecution(
        req, qos, publish_complete(job->get_interface(), callback, on_publish_completed_job));
}

void drop_job(InternalJob *job)
{
    delete job;
}

jobs::OnSubscribeToDescribeJobExecutionAcceptedResponse describe_job_execution_accepted(const void *interface)
{
    return [=](jobs::DescribeJobExecutionResponse *response, int32_t io_err)
    {
        auto info = std::unique_ptr<JobInfo>(nullptr);
        char *client_token = nullptr;

        if (response->Execution)
        {
            info = get_job_info(&*response->Execution);
        }

        if (response->ClientToken)
        {
            client_token = const_cast<char *>(response->ClientToken->c_str());
        }

        on_describe_job_execution_accepted(interface, client_token, info.get(), io_err);
    };
}

jobs::OnSubscribeToUpdateJobExecutionAcceptedResponse update_job_execution_accepted(const void *interface)
{
    return [=](jobs::UpdateJobExecutionResponse *response, int32_t io_err)
    {
        auto info = std::make_unique<JobInfo>();
        char *client_token = nullptr;

        if (response->JobDocument)
        {
            AwsString json = response->JobDocument->View().WriteCompact();
            auto buff = Buffer::create(json.size());

            if (!buff.is_empty())
            {
                std::memcpy(buff.data, json.data(), json.size());
                info->job_document = std::move(buff);
            }
        }

        on_update_job_execution_accepted(interface, client_token, info.get(), io_err);
    };
}
