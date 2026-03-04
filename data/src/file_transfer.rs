use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};

pub use self::manager::Manager;
pub use self::task::Task;
use crate::{Server, User, dcc, server};

pub mod manager;
pub mod task;

const DEFAULT_RECEIVE_FILENAME: &str = "download";

pub fn sanitize_received_filename(filename: &str) -> String {
    let Some(candidate) =
        filename.rsplit(['/', '\\']).find(|part| !part.is_empty())
    else {
        return DEFAULT_RECEIVE_FILENAME.to_string();
    };

    let sanitized = candidate
        .chars()
        .filter(|c| *c != '\0' && !c.is_control())
        .collect::<String>();

    if sanitized.is_empty() {
        return DEFAULT_RECEIVE_FILENAME.to_string();
    }

    let mut components = Path::new(&sanitized).components();

    if matches!(components.next(), Some(Component::Normal(_)))
        && components.next().is_none()
    {
        sanitized
    } else {
        DEFAULT_RECEIVE_FILENAME.to_string()
    }
}

pub fn received_file_save_path(
    save_directory: &Path,
    filename: &str,
) -> PathBuf {
    save_directory.join(sanitize_received_filename(filename))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(u16);

impl From<u16> for Id {
    fn from(value: u16) -> Self {
        Id(value)
    }
}

impl From<Id> for u16 {
    fn from(id: Id) -> Self {
        id.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTransfer {
    pub id: Id,
    pub server: Server,
    pub created_at: DateTime<Utc>,
    pub direction: Direction,
    pub remote_user: User,
    pub filename: String,
    pub size: u64,
    pub status: Status,
}

impl FileTransfer {
    pub fn progress(&self) -> f64 {
        match self.status {
            Status::Active { transferred, .. } => {
                transferred as f64 / self.size as f64
            }
            Status::Completed { .. } => 1.0,
            _ => 0.0,
        }
    }
}

impl PartialOrd for FileTransfer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FileTransfer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.created_at
            .cmp(&other.created_at)
            .reverse()
            .then_with(|| self.direction.cmp(&other.direction))
            .then_with(|| {
                self.remote_user
                    .nickname()
                    .cmp(&other.remote_user.nickname())
            })
            .then_with(|| self.filename.cmp(&other.filename))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Sent,
    Received,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// Pending approval
    PendingApproval,
    /// Pending reverse confirmation
    PendingReverseConfirmation,
    /// Queued (needs an open port to begin)
    Queued,
    /// Ready (waiting for remote user to connect)
    Ready,
    /// Transfer is actively sending / receiving
    Active { transferred: u64, elapsed: Duration },
    /// Transfer is complete
    Completed { elapsed: Duration, sha256: String },
    /// An error occurred
    Failed { error: String },
}

#[derive(Debug, Clone)]
pub struct ReceiveRequest {
    pub from: User,
    pub dcc_send: dcc::Send,
    pub server: Server,
    pub server_handle: server::Handle,
}

#[derive(Debug)]
pub struct SendRequest {
    pub to: User,
    pub path: PathBuf,
    pub server: Server,
    pub server_handle: server::Handle,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{received_file_save_path, sanitize_received_filename};

    #[test]
    fn sanitize_received_filename_removes_path_segments() {
        assert_eq!(sanitize_received_filename("../../etc/passwd"), "passwd");
        assert_eq!(
            sanitize_received_filename("..\\..\\AppData\\evil.txt"),
            "evil.txt"
        );
        assert_eq!(
            sanitize_received_filename("/tmp/../../target.txt"),
            "target.txt"
        );
    }

    #[test]
    fn sanitize_received_filename_falls_back_for_invalid_components() {
        assert_eq!(sanitize_received_filename(".."), "download");
        assert_eq!(sanitize_received_filename("/"), "download");
        assert_eq!(sanitize_received_filename(""), "download");
    }

    #[test]
    fn received_file_save_path_stays_in_directory() {
        let save_directory = PathBuf::from("downloads");

        assert_eq!(
            received_file_save_path(&save_directory, "../../etc/passwd"),
            save_directory.join("passwd")
        );
        assert_eq!(
            received_file_save_path(&save_directory, "/absolute/path.bin"),
            save_directory.join("path.bin")
        );
        assert_eq!(
            received_file_save_path(&save_directory, "..\\..\\oops.dat"),
            save_directory.join("oops.dat")
        );
    }
}
