#pragma once
#include <aws/iotjobs/DescribeJobExecutionRequest.h>
#include <aws/iotjobs/DescribeJobExecutionResponse.h>
#include <aws/iotjobs/DescribeJobExecutionSubscriptionRequest.h>
#include <aws/iotjobs/GetPendingJobExecutionsResponse.h>
#include <aws/iotjobs/GetPendingJobExecutionsRequest.h>
#include <aws/iotjobs/GetPendingJobExecutionsSubscriptionRequest.h>
#include <aws/iotjobs/JobExecutionsChangedSubscriptionRequest.h>
#include <aws/iotjobs/NextJobExecutionChangedSubscriptionRequest.h>
#include <aws/iotjobs/NextJobExecutionChangedEvent.h>
#include <aws/iotjobs/UpdateJobExecutionResponse.h>
#include <aws/iotjobs/JobExecutionsChangedEvent.h>
#include <aws/iotjobs/IotJobsClient.h>
#include <aws/iotjobs/JobExecutionSummary.h>
#include <aws/iotjobs/RejectedError.h>
#include <aws/iotjobs/StartNextJobExecutionResponse.h>
#include <aws/iotjobs/StartNextPendingJobExecutionRequest.h>
#include <aws/iotjobs/StartNextPendingJobExecutionSubscriptionRequest.h>
#include <aws/iotjobs/UpdateJobExecutionRequest.h>
#include <aws/iotjobs/UpdateJobExecutionSubscriptionRequest.h>
#include "mqtt.h"
#include "common.h"
#include "logs.h"

namespace jobs = Aws::Iotjobs;
namespace crt = Aws::Crt;

extern "C"
{
    class InternalJobsClient final
    {
    private:
        [[maybe_unused]] std::shared_ptr<jobs::IotJobsClient> client;
        const void *interface;
        AwsString thing_name;

    public:
        InternalJobsClient(std::shared_ptr<jobs::IotJobsClient> client, const void *interface, const char *thing_name);
        std::shared_ptr<jobs::IotJobsClient> internal_client();
        const void *get_interface() const;
        AwsString get_name() const;
    };

    class InternalJob final
    {
    private:
        [[maybe_unused]] std::shared_ptr<jobs::IotJobsClient> client;
        const void *interface;
        AwsString thing_name;
        AwsString job_id;

    public:
        InternalJob(std::shared_ptr<jobs::IotJobsClient> client, const void *interface, const char *thing_name, const char *job_id);
        const void *get_interface() const;
        std::shared_ptr<jobs::IotJobsClient> internal_client();
        AwsString get_name() const;
        AwsString get_job_id() const;
    };

    struct JobInfo
    {
        explicit JobInfo();
        char *job_id;
        Buffer job_document;
        jobs::JobStatus *status;
        int32_t *version_number;
        crt::DateTime *queue_at;
        char *thing_name;
        int64_t *execution_number;
        crt::DateTime *last_updated_at;
        crt::DateTime *started_at;
    };

    struct JobExecutionSummary
    {
        explicit JobExecutionSummary();
        explicit JobExecutionSummary(
            const char *, const int32_t *,
            const int64_t *, const crt::DateTime *,
            const crt::DateTime *, const crt::DateTime *);
        const char *job_id;
        const int32_t *version_number;
        const int64_t *execution_number;
        const crt::DateTime *started_at;
        const crt::DateTime *queued_at;
        const crt::DateTime *last_updated_at;
    };

    struct JobsSummary
    {
        explicit JobsSummary();
        explicit JobsSummary(JobExecutionSummary *, JobExecutionSummary *, size_t, size_t);
        JobExecutionSummary *queued_jobs;
        JobExecutionSummary *progress_jobs;
        size_t queued_size;
        size_t progres_size;
    };

    struct Rejected
    {
        explicit Rejected();
        crt::DateTime *timestamp;
        jobs::RejectedErrorCode *code;
        char *message;
        char *client_token;
    };

    struct DescribeExecutionRequest
    {
        int64_t *execution_number;
        bool *include_document;
        char *job_id;
    };

    struct NextPendingRequest
    {
        /**
         * Specifies the amount of time this device has to finish execution of this job in minutes.
         *
         */
        int64_t *step_timeout;
    };

    struct UpdateExecutionRequest
    {
        int64_t *execution_number;
        bool *include_execution_state;
        char *job_id;
        int32_t *expected_version;
        bool *include_document;
        jobs::JobStatus *status;
        /**
         * Specifies the amount of time this device has to finish execution of this job in minutes.
         *
         */
        int64_t *step_timeout;
    };

    InternalJobsClient *internal_jobs_client(
        InternalMqttClient *mqtt_client, const void *interface,
        QOS qos, const char *thing_name);

    /**
     * Publishes a request to get the next pending job execution for the given client.
     *
     * @param client The InternalJobsClient instance.
     * @param qos The MQTT QOS to use for the request.
     * @param callback The callback to invoke when the response is received.
     */
    bool publish_get_pending_executions(InternalJobsClient *client, QOS qos, const void *callback);
    /**
     * Publishes a request to start executing the next pending job for this device.
     *
     * @param client The InternalJobsClient instance.
     * @param qos The MQTT QOS to use for the request.
     * @param callback The callback to invoke when a response is received.
     * @param request Additional parameters for the request.
     * @return True if publish succeeded, false otherwise.
     */
    bool publish_start_next_pending_execution(InternalJobsClient *client, QOS qos, const void *callback, NextPendingRequest request);
    void drop_jobs_client(InternalJobsClient *client);

    InternalJob *internal_job(
        InternalMqttClient *mqtt_client,
        const void *interface, QOS qos,
        const char *thing_name, const char *job_id);
    /**
     * Publishes a DescribeExecution request for the given job to get details about a specific execution.
     *
     * @param job The job to publish the request for.
     * @param qos The QoS to use for the publish.
     * @param callback The callback to invoke when the request is complete.
     * @param request The details of the execution to get information about.
     * @return True if the request was published successfully, false otherwise.
     */
    bool publish_describe_execution(InternalJob *job, QOS qos, const void *callback, DescribeExecutionRequest request);

    /**
     * Publishes an UpdateExecution request for the given job to update details about a specific execution.
     *
     * @param job The job to publish the request for.
     * @param qos The QoS to use for the publish.
     * @param callback The callback to invoke when the request is complete.
     * @param request The details of the execution to update.
     */
    bool publish_update_execution(InternalJob *job, QOS qos, const void *callback, UpdateExecutionRequest request);
    void drop_job(InternalJob *job);
}

/**
 * Callback function called when subscribe operation completes.
 */
jobs::OnSubscribeComplete subscribe_completed(const void *, std::function<void(const void *, int32_t)>);
/**
 * Callback function called when subscribe operation completes.
 */
jobs::OnSubscribeComplete publish_complete(const void *, const void *, std::function<void(const void *, const void *, int32_t)>);
/**
 * Callback function called when a job execution is rejected.
 */
std::function<void(Aws::Iotjobs::RejectedError *, int32_t ioErr)> rejected(const void *, std::function<void(const void *, Rejected, int32_t)>);
/**
 * Gets job information from job execution data.
 */
std::unique_ptr<JobInfo> get_job_info(jobs::JobExecutionData *data);
