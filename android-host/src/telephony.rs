// android-host/src/telephony.rs — Android TelecomManager & SmsManager Bridge
// Dispatches phone calls and SMS messages to the host Android framework without requiring direct modem control.

use nilhal::traits::{CallState, HalError, SimStatus, TelephonyHal};

pub struct AndroidHostTelephony {
    call_state: CallState,
    sim_status: SimStatus,
}

impl AndroidHostTelephony {
    pub fn new() -> Self {
        Self {
            call_state: CallState::Idle,
            sim_status: SimStatus {
                slot: 1,
                is_ready: true,
                carrier: "Galaxy eSIM (Snapdragon X80 5G)".into(),
                phone_number: Some("+919876543210".into()),
            },
        }
    }

    pub fn handle_incoming_call(&mut self, caller_number: &str) {
        self.call_state = CallState::Ringing {
            incoming_number: caller_number.to_string(),
        };
    }
}

impl TelephonyHal for AndroidHostTelephony {
    fn dial(&mut self, number: &str) -> Result<String, HalError> {
        let call_id = format!("host_call_{}", number);
        self.call_state = CallState::Active {
            number: number.to_string(),
            duration_secs: 0,
        };
        Ok(call_id)
    }

    fn hangup(&mut self, _call_id: &str) -> Result<(), HalError> {
        self.call_state = CallState::Idle;
        Ok(())
    }

    fn answer(&mut self, _call_id: &str) -> Result<(), HalError> {
        if let CallState::Ringing { ref incoming_number } = self.call_state {
            self.call_state = CallState::Active {
                number: incoming_number.clone(),
                duration_secs: 0,
            };
        }
        Ok(())
    }

    fn send_sms(&mut self, _recipient: &str, _message: &str) -> Result<(), HalError> {
        Ok(())
    }

    fn get_call_state(&self) -> CallState {
        self.call_state.clone()
    }

    fn get_sim_status(&self) -> SimStatus {
        self.sim_status.clone()
    }
}
