use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct LogcatCursorState {
    pub id: String,
    pub device: Option<String>,
    pub filter: Option<String>,
    pub lines: Vec<String>,
    pub current_offset: usize,
    pub page_size: usize,
    pub created_at: Instant,
    pub last_accessed: Instant,
}

impl LogcatCursorState {
    pub fn new(
        device: Option<String>,
        filter: Option<String>,
        logs: String,
        page_size: usize,
    ) -> Self {
        let lines: Vec<String> = logs.lines().map(|s| s.to_string()).collect();
        let id = Uuid::new_v4().to_string();
        let now = Instant::now();

        Self {
            id,
            device,
            filter,
            lines,
            current_offset: 0,
            page_size,
            created_at: now,
            last_accessed: now,
        }
    }

    pub fn get_next_page(&mut self) -> Option<(Vec<String>, bool)> {
        self.last_accessed = Instant::now();

        if self.current_offset >= self.lines.len() {
            return None;
        }

        let end = std::cmp::min(self.current_offset + self.page_size, self.lines.len());
        let page = self.lines[self.current_offset..end].to_vec();
        let has_more = end < self.lines.len();

        self.current_offset = end;

        Some((page, has_more))
    }

    pub fn get_first_page(&mut self) -> (Vec<String>, bool) {
        self.current_offset = 0;
        self.last_accessed = Instant::now();

        let end = std::cmp::min(self.page_size, self.lines.len());
        let page = self.lines[0..end].to_vec();
        let has_more = end < self.lines.len();

        self.current_offset = end;

        (page, has_more)
    }

    pub fn is_stale(&self, timeout: Duration) -> bool {
        self.last_accessed.elapsed() > timeout
    }
}

pub struct LogcatCursorManager {
    cursors: Arc<DashMap<String, LogcatCursorState>>,
    cleanup_interval: Duration,
    timeout: Duration,
}

impl LogcatCursorManager {
    pub fn new(timeout_secs: u64, cleanup_interval_secs: u64) -> Self {
        let manager = Self {
            cursors: Arc::new(DashMap::new()),
            timeout: Duration::from_secs(timeout_secs),
            cleanup_interval: Duration::from_secs(cleanup_interval_secs),
        };

        // Start cleanup task
        let cursors_clone = Arc::clone(&manager.cursors);
        let timeout = manager.timeout;
        let interval = manager.cleanup_interval;

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                cursors_clone.retain(|_, cursor| !cursor.is_stale(timeout));
            }
        });

        manager
    }

    pub fn create_cursor(
        &self,
        device: Option<String>,
        filter: Option<String>,
        logs: String,
        page_size: usize,
    ) -> String {
        let cursor = LogcatCursorState::new(device, filter, logs, page_size);
        let cursor_id = cursor.id.clone();
        self.cursors.insert(cursor_id.clone(), cursor);
        cursor_id
    }

    pub fn get_next_page(
        &self,
        cursor_id: &str,
    ) -> Result<(Vec<String>, String, bool, usize, usize), String> {
        match self.cursors.get_mut(cursor_id) {
            Some(mut cursor) => match cursor.get_next_page() {
                Some((page, has_more)) => {
                    let total = cursor.lines.len();
                    let offset = cursor.current_offset;
                    Ok((page, cursor_id.to_string(), has_more, offset, total))
                }
                None => Err("Cursor has reached the end of logs".to_string()),
            },
            None => Err(format!("Cursor {} not found or expired", cursor_id)),
        }
    }

    pub fn get_first_page(
        &self,
        device: Option<String>,
        filter: Option<String>,
        logs: String,
        page_size: usize,
    ) -> (Vec<String>, String, bool, usize, usize) {
        let cursor_id = self.create_cursor(device, filter, logs.clone(), page_size);

        match self.cursors.get_mut(&cursor_id) {
            Some(mut cursor) => {
                let (page, has_more) = cursor.get_first_page();
                let total = cursor.lines.len();
                let offset = cursor.current_offset;
                (page, cursor_id, has_more, offset, total)
            }
            None => (vec![], cursor_id, false, 0, 0),
        }
    }
}
