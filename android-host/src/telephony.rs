// android-host/src/telephony.rs — Android TelecomManager & SmsManager Bridge
// Dispatches phone calls and SMS messages to the host Android framework without requiring direct modem control.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::bridge::{GuestToHostCommand, HostToGuestEvent};
use crate::jni_bridge;
use nilhal::traits::{CallState, HalError, SimStatus, SmsMessage, TelephonyHal};

pub struct AndroidHostTelephony {
    call_state: CallState,
    sim_status: SimStatus,
    active_call_id: Option<String>,
    sent_messages: Vec<SmsMessage>,
    received_messages: Vec<SmsMessage>,
}

impl AndroidHostTelephony {
    pub fn new() -> Self {
        Self {
            call_state: CallState::Idle,
            sim_status: SimStatus {
                slot: 1,
                is_ready: true,
                carrier: "Android Host Telecom Bridge".into(),
                phone_number: None,
            },
            active_call_id: None,
            sent_messages: Vec::new(),
            received_messages: Vec::new(),
        }
    }

    /// Process telephony events received from Android BroadcastReceivers
    pub fn handle_host_event(&mut self, event: &HostToGuestEvent) {
        match event {
            HostToGuestEvent::IncomingCall { call_id, caller_number } => {
                self.active_call_id = Some(call_id.clone());
                self.call_state = CallState::Ringing {
                    incoming_number: caller_number.clone(),
                };
            }
            HostToGuestEvent::CallStateChanged { call_id, state } => {
                match state.as_str() {
                    "ACTIVE" => {
                        let num = match &self.call_state {
                            CallState::Ringing { incoming_number } => incoming_number.clone(),
                            CallState::Active { number, .. } => number.clone(),
                            _ => "Unknown".to_string(),
                        };
                        self.active_call_id = Some(call_id.clone());
                        self.call_state = CallState::Active {
                            number: num,
                            duration_secs: 0,
                        };
                    }
                    "DISCONNECTED" | "IDLE" => {
                        self.active_call_id = None;
                        self.call_state = CallState::Idle;
                    }
                    _ => {}
                }
            }
            HostToGuestEvent::SmsReceived { sender, body, timestamp } => {
                self.received_messages.push(SmsMessage {
                    sender: sender.clone(),
                    body: body.clone(),
                    timestamp: *timestamp,
                });
            }
            HostToGuestEvent::NetworkUpdate { conn_type, .. }
                if conn_type.contains("5G") || conn_type.contains("CELLULAR") =>
            {
                self.sim_status.is_ready = true;
                self.sim_status.carrier = format!("Cellular Bridge ({})", conn_type);
            }
            _ => {}
        }
    }

    pub fn set_sim_info(&mut self, carrier: &str, phone_number: Option<String>) {
        self.sim_status.carrier = carrier.to_string();
        self.sim_status.phone_number = phone_number;
    }

    pub fn get_sent_messages(&self) -> &[SmsMessage] {
        &self.sent_messages
    }

    pub fn get_received_messages(&self) -> &[SmsMessage] {
        &self.received_messages
    }
}

impl Default for AndroidHostTelephony {
    fn default() -> Self {
        Self::new()
    }
}

impl TelephonyHal for AndroidHostTelephony {
    fn dial(&mut self, number: &str) -> Result<String, HalError> {
        let trimmed = number.trim();
        if trimmed.is_empty() {
            return Err(HalError::UnsupportedOperation(
                "Cannot dial an empty phone number".into(),
            ));
        }

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let call_id = format!("call_{}_{}", trimmed.replace(['+', '-', ' '], ""), now_ms);

        self.call_state = CallState::Active {
            number: trimmed.to_string(),
            duration_secs: 0,
        };
        self.active_call_id = Some(call_id.clone());

        // Forward to Android host to trigger TelecomManager.placeCall() / Intent.ACTION_DIAL
        jni_bridge::enqueue_guest_command(GuestToHostCommand::DialNumber {
            number: trimmed.to_string(),
        });

        Ok(call_id)
    }

    fn hangup(&mut self, call_id: &str) -> Result<(), HalError> {
        self.call_state = CallState::Idle;
        self.active_call_id = None;

        jni_bridge::enqueue_guest_command(GuestToHostCommand::HangupCall {
            call_id: call_id.to_string(),
        });

        Ok(())
    }

    fn answer(&mut self, call_id: &str) -> Result<(), HalError> {
        if let CallState::Ringing { ref incoming_number } = self.call_state {
            self.call_state = CallState::Active {
                number: incoming_number.clone(),
                duration_secs: 0,
            };
        } else {
            self.call_state = CallState::Active {
                number: "Unknown".into(),
                duration_secs: 0,
            };
        }

        jni_bridge::enqueue_guest_command(GuestToHostCommand::AnswerCall {
            call_id: call_id.to_string(),
        });

        Ok(())
    }

    fn send_sms(&mut self, recipient: &str, message: &str) -> Result<(), HalError> {
        let recipient_trim = recipient.trim();
        let message_trim = message.trim();

        if recipient_trim.is_empty() {
            return Err(HalError::UnsupportedOperation("SMS recipient cannot be empty".into()));
        }
        if message_trim.is_empty() {
            return Err(HalError::UnsupportedOperation("SMS message body cannot be empty".into()));
        }

        let now_sec = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.sent_messages.push(SmsMessage {
            sender: "self".to_string(),
            body: message_trim.to_string(),
            timestamp: now_sec,
        });

        // Forward to Android host to trigger SmsManager.getDefault().sendTextMessage()
        jni_bridge::enqueue_guest_command(GuestToHostCommand::SendSms {
            recipient: recipient_trim.to_string(),
            message: message_trim.to_string(),
        });

        Ok(())
    }

    fn get_call_state(&self) -> CallState {
        self.call_state.clone()
    }

    fn get_sim_status(&self) -> SimStatus {
        self.sim_status.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telephony_dial_and_hangup() {
        let _guard = crate::jni_bridge::test_lock();
        let mut tel = AndroidHostTelephony::new();
        assert_eq!(tel.get_call_state(), CallState::Idle);

        // Empty number fails
        assert!(tel.dial("").is_err());

        // Valid dial
        let call_id = tel.dial("+919876543210").expect("Dial should succeed");
        assert!(call_id.starts_with("call_919876543210"));
        match tel.get_call_state() {
            CallState::Active { number, .. } => assert_eq!(number, "+919876543210"),
            _ => panic!("Expected active call state"),
        }

        // Hangup
        assert!(tel.hangup(&call_id).is_ok());
        assert_eq!(tel.get_call_state(), CallState::Idle);
    }

    #[test]
    fn test_telephony_sms_pipeline() {
        let _guard = crate::jni_bridge::test_lock();
        let mut tel = AndroidHostTelephony::new();

        assert!(tel.send_sms("", "hello").is_err());
        assert!(tel.send_sms("+919876543210", "").is_err());

        assert!(tel.send_sms("+919876543210", "Test from Onuron").is_ok());
        assert_eq!(tel.get_sent_messages().len(), 1);
        assert_eq!(tel.get_sent_messages()[0].body, "Test from Onuron");

        // Simulate incoming SMS
        tel.handle_host_event(&HostToGuestEvent::SmsReceived {
            sender: "+911234567890".into(),
            body: "Welcome to OnuronOS".into(),
            timestamp: 1234567,
        });
        assert_eq!(tel.get_received_messages().len(), 1);
        assert_eq!(tel.get_received_messages()[0].sender, "+911234567890");
    }

    #[test]
    fn test_incoming_call_and_answer() {
        let _guard = crate::jni_bridge::test_lock();
        let mut tel = AndroidHostTelephony::new();
        tel.handle_host_event(&HostToGuestEvent::IncomingCall {
            call_id: "inc_1".into(),
            caller_number: "+919999999999".into(),
        });

        match tel.get_call_state() {
            CallState::Ringing { incoming_number } => assert_eq!(incoming_number, "+919999999999"),
            _ => panic!("Expected ringing state"),
        }

        assert!(tel.answer("inc_1").is_ok());
        match tel.get_call_state() {
            CallState::Active { number, .. } => assert_eq!(number, "+919999999999"),
            _ => panic!("Expected active state after answer"),
        }
    }
}
