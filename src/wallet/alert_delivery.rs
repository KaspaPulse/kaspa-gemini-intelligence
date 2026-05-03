#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertDeliveryAttempt {
    SendSucceeded,
    SendFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertDeliveryOutcome {
    Delivered,
    Failed,
}

pub fn delivery_outcome(attempt: AlertDeliveryAttempt) -> AlertDeliveryOutcome {
    match attempt {
        AlertDeliveryAttempt::SendSucceeded => AlertDeliveryOutcome::Delivered,
        AlertDeliveryAttempt::SendFailed => AlertDeliveryOutcome::Failed,
    }
}

pub fn should_record_delivered(outcome: AlertDeliveryOutcome) -> bool {
    matches!(outcome, AlertDeliveryOutcome::Delivered)
}

pub fn should_record_failed(outcome: AlertDeliveryOutcome) -> bool {
    matches!(outcome, AlertDeliveryOutcome::Failed)
}
