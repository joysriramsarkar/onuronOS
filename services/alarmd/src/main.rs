// services/alarmd/src/main.rs — RTC Wake Alarm Scheduler
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlarmEntry {
    pub id: u64,
    pub fire_at_epoch: u64,
    pub label: String,
    pub wake_device: bool,
}

#[derive(Debug, Default)]
pub struct AlarmScheduler {
    alarms: Vec<AlarmEntry>,
    next_id: u64,
}

impl AlarmScheduler {
    pub fn new() -> Self {
        Self {
            alarms: Vec::new(),
            next_id: 1,
        }
    }

    pub fn schedule(
        &mut self,
        fire_at_epoch: u64,
        label: impl Into<String>,
        wake_device: bool,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.alarms.push(AlarmEntry {
            id,
            fire_at_epoch,
            label: label.into(),
            wake_device,
        });
        self.alarms.sort_by_key(|a| a.fire_at_epoch);
        id
    }

    pub fn cancel(&mut self, id: u64) -> bool {
        let prev = self.alarms.len();
        self.alarms.retain(|a| a.id != id);
        self.alarms.len() < prev
    }

    pub fn next_wake_epoch(&self) -> Option<u64> {
        self.alarms
            .iter()
            .filter(|a| a.wake_device)
            .map(|a| a.fire_at_epoch)
            .min()
    }

    pub fn pop_due(&mut self, now_epoch: u64) -> Vec<AlarmEntry> {
        let mut due = Vec::new();
        let mut remaining = Vec::new();
        for a in self.alarms.drain(..) {
            if a.fire_at_epoch <= now_epoch {
                due.push(a);
            } else {
                remaining.push(a);
            }
        }
        self.alarms = remaining;
        due
    }

    pub fn sync_rtc_wakealarm(&self, rtc_path: &Path) -> std::io::Result<()> {
        if !rtc_path.exists() {
            return Ok(());
        }
        // Reset existing wakealarm before programming the next earliest epoch
        let _ = fs::write(rtc_path, "0");
        if let Some(epoch) = self.next_wake_epoch() {
            fs::write(rtc_path, epoch.to_string())?;
        }
        Ok(())
    }
}

fn main() {
    println!("[alarmd] RTC Wake Alarm Scheduler active.");
    let _ = nilsd::notify_ready("alarmd", None);
    let mut scheduler = AlarmScheduler::new();
    let rtc_path = Path::new("/sys/class/rtc/rtc0/wakealarm");

    loop {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        for fired in scheduler.pop_due(now) {
            println!("[alarmd] Alarm #{} fired: {}", fired.id, fired.label);
        }
        let _ = scheduler.sync_rtc_wakealarm(rtc_path);
        thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alarm_ordering_and_due_firing() {
        let mut sched = AlarmScheduler::new();
        let id1 = sched.schedule(2000, "Later", true);
        let id2 = sched.schedule(1000, "Sooner", true);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(sched.next_wake_epoch(), Some(1000));

        let due = sched.pop_due(1500);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].label, "Sooner");
        assert_eq!(sched.next_wake_epoch(), Some(2000));

        assert!(sched.cancel(id1));
        assert_eq!(sched.next_wake_epoch(), None);
    }
}
