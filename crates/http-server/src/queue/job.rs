use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Priority levels for job processing.
///
/// Higher priority jobs can cancel existing lower priority jobs for the same workspace.
/// This ensures user-triggered or high-priority operations take precedence over background tasks.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub enum JobPriority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
}

/// Job types that can be processed by the queue system.
///
/// Each job variant represents a different type of work that can be performed.
/// Jobs are routed to workspace-specific queues for sequential processing.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
#[serde(tag = "type", content = "data")]
pub enum Job {
    /// This job triggers a full indexing of all Git repositories found within
    /// the specified workspace folder path.
    IndexWorkspaceFolder {
        workspace_folder_path: String,
        priority: JobPriority,
    },
}

impl Job {
    pub fn workspace_path(&self) -> &str {
        match self {
            Job::IndexWorkspaceFolder {
                workspace_folder_path,
                ..
            } => workspace_folder_path,
        }
    }

    pub fn priority(&self) -> JobPriority {
        match self {
            Job::IndexWorkspaceFolder { priority, .. } => priority.clone(),
        }
    }

    pub fn job_type(&self) -> &'static str {
        match self {
            Job::IndexWorkspaceFolder { .. } => "IndexWorkspaceFolder",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct JobInfo {
    pub id: String,
    pub job: Job,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: JobStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_priority_ordering() {
        assert!(JobPriority::High > JobPriority::Normal);
        assert!(JobPriority::Normal > JobPriority::Low);
        assert_eq!(JobPriority::default(), JobPriority::Normal);
    }

    #[test]
    fn test_job_workspace_path_extraction() {
        let job = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/test/path".to_string(),
            priority: JobPriority::High,
        };

        assert_eq!(job.workspace_path(), "/test/path");
        assert_eq!(job.priority(), JobPriority::High);
        assert_eq!(job.job_type(), "IndexWorkspaceFolder");
    }

    #[test]
    fn test_job_info_serialization() {
        let job = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/test/path".to_string(),
            priority: JobPriority::High,
        };

        let job_info = JobInfo {
            id: "test-job-id".to_string(),
            job: job.clone(),
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            status: JobStatus::Pending,
            error: None,
        };

        let serialized = serde_json::to_string(&job_info).unwrap();
        let deserialized: JobInfo = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, "test-job-id");
        assert_eq!(deserialized.status, JobStatus::Pending);
        assert_eq!(deserialized.job.workspace_path(), "/test/path");
    }

    #[test]
    fn test_job_status_equality() {
        assert_eq!(JobStatus::Pending, JobStatus::Pending);
        assert_ne!(JobStatus::Pending, JobStatus::Running);
        assert_ne!(JobStatus::Running, JobStatus::Completed);
    }

    #[test]
    fn test_job_priority_serialization() {
        let priorities = vec![JobPriority::Low, JobPriority::Normal, JobPriority::High];

        for priority in priorities {
            let serialized = serde_json::to_string(&priority).unwrap();
            let deserialized: JobPriority = serde_json::from_str(&serialized).unwrap();
            assert_eq!(priority, deserialized);
        }
    }

    #[test]
    fn test_job_serialization() {
        let job = Job::IndexWorkspaceFolder {
            workspace_folder_path: "/workspace/path".to_string(),
            priority: JobPriority::Normal,
        };

        let serialized = serde_json::to_string(&job).unwrap();
        let deserialized: Job = serde_json::from_str(&serialized).unwrap();

        assert_eq!(job.workspace_path(), deserialized.workspace_path());
        assert_eq!(job.priority(), deserialized.priority());
        assert_eq!(job.job_type(), deserialized.job_type());
    }
}
