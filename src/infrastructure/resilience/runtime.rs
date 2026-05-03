use crate::domain::errors::AppError;
use std::future::Future;
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration};

pub fn env_u64(key: &str, default_value: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default_value)
}

pub fn rpc_timeout_duration() -> Duration {
    Duration::from_secs(env_u64("RPC_TIMEOUT_SECS", 15))
}

pub fn http_timeout_duration() -> Duration {
    Duration::from_secs(env_u64("HTTP_TIMEOUT_SECS", 10))
}

pub async fn with_rpc_timeout<T, F>(operation: &'static str, future: F) -> Result<T, AppError>
where
    F: Future<Output = Result<T, AppError>>,
{
    match timeout(rpc_timeout_duration(), future).await {
        Ok(result) => result,
        Err(_) => Err(AppError::NodeConnection(format!(
            "RPC timeout while running {} after {} seconds",
            operation,
            rpc_timeout_duration().as_secs()
        ))),
    }
}

pub async fn with_timeout_result<T, E, F>(
    operation: &'static str,
    duration: Duration,
    future: F,
) -> Result<T, String>
where
    E: std::fmt::Display,
    F: Future<Output = Result<T, E>>,
{
    match timeout(duration, future).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(format!("{} failed: {}", operation, error)),
        Err(_) => Err(format!(
            "{} timed out after {} seconds",
            operation,
            duration.as_secs()
        )),
    }
}

pub fn spawn_resilient<F>(task_name: &'static str, future: F) -> JoinHandle<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    let worker = tokio::spawn(async move {
        tracing::info!("[TASK START] {}", task_name);
        future.await;
        tracing::info!("[TASK STOP] {} finished normally", task_name);
    });

    tokio::spawn(async move {
        match worker.await {
            Ok(_) => {
                tracing::info!("[TASK MONITOR] {} joined cleanly", task_name);
            }
            Err(error) if error.is_panic() => {
                tracing::error!(
                    "[TASK PANIC] {} crashed with panic. Global panic hook should record marker if process-level panic occurs.",
                    task_name
                );
            }
            Err(error) if error.is_cancelled() => {
                tracing::warn!("[TASK CANCELLED] {} was cancelled", task_name);
            }
            Err(error) => {
                tracing::error!("[TASK ERROR] {} join error: {}", task_name, error);
            }
        }
    })
}
