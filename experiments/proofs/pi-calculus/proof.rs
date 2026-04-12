// proof.rs — emitted by: loom compile proof.loom
// Theory: π-Calculus (Milner 1992)
// Channels are first-class values. Sending a channel on a channel = mobile communication.
// Ecosystem signal routing is verified at compile time via typed channels.

use std::sync::mpsc;

/// Typed channel for mobile communication (π-calculus: channels as values)
pub type SignalChannel<T> = mpsc::Sender<T>;

// ── Beings (processes in π-calculus terms) ───────────────────────────────────

pub struct TemperatureSensor { pub reading: f64 }
pub struct CoolingActuator { pub cooling_power: f64 }
pub struct ForecastModel { pub forecast_accuracy: f64 }

impl TemperatureSensor {
    /// Emit temperature reading on the shared channel
    pub fn emit(&self, channel: &SignalChannel<f64>) {
        channel.send(self.reading).ok();
    }
}

impl CoolingActuator {
    /// Receive temperature signal and adjust cooling
    pub fn receive(&mut self, temperature: f64) {
        self.cooling_power = if temperature > 25.0 {
            (self.cooling_power + 0.1).min(1.0)
        } else {
            (self.cooling_power - 0.05).max(0.0)
        };
    }

    /// π-calculus mobile channel: receive a NEW channel from ForecastModel
    pub fn receive_control_channel(&self, control: SignalChannel<f64>) -> SignalChannel<f64> {
        control // now uses the dynamically received channel
    }
}

impl ForecastModel {
    /// Send a new control channel to CoolingActuator (mobile channel communication)
    pub fn send_control_channel(
        &self,
        actuator_channel: &SignalChannel<SignalChannel<f64>>,
    ) -> mpsc::Receiver<f64> {
        let (tx, rx) = mpsc::channel::<f64>();
        actuator_channel.send(tx).ok();
        rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temperature_signal_routing() {
        let (tx, rx) = mpsc::channel::<f64>();
        let sensor = TemperatureSensor { reading: 28.0 };
        sensor.emit(&tx);
        let received = rx.recv().unwrap();
        assert_eq!(received, 28.0, "signal must be routed correctly");
    }

    #[test]
    fn actuator_responds_to_temperature_signal() {
        let mut actuator = CoolingActuator { cooling_power: 0.5 };
        actuator.receive(30.0); // hot -> increase cooling
        assert!(actuator.cooling_power > 0.5, "actuator must increase cooling for hot signal");
    }

    #[test]
    fn mobile_channel_pi_calculus_pattern() {
        // π-calculus: ForecastModel sends a NEW channel to CoolingActuator
        let (control_tx, control_rx) = mpsc::channel::<SignalChannel<f64>>();
        let model = ForecastModel { forecast_accuracy: 0.9 };
        let _rx = model.send_control_channel(&control_tx);

        // CoolingActuator receives the channel (mobile communication)
        let received_channel = control_rx.recv().unwrap();
        let actuator = CoolingActuator { cooling_power: 0.5 };
        let _new_channel = actuator.receive_control_channel(received_channel);
        // Channel mobility: the routing graph changed at runtime, but the types are still safe
    }
}
