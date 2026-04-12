// proof.rs — emitted by: loom compile proof.loom
// Theory: Temporal Logic (Pnueli 1977)
// before:/after: constraints become runtime-checked ordering guards.
// Lifecycle state machines enforce valid transition sequences.

#[derive(Debug, Clone, PartialEq)]
pub enum OrderState {
    Pending,
    Paid,
    Shipped,
    Delivered,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub state: OrderState,
    pub id: u64,
}

impl Order {
    pub fn new(id: u64) -> Self {
        Self { state: OrderState::Pending, id }
    }

    /// Temporal invariant: Paid only reachable from Pending
    pub fn confirm_payment(&mut self) -> Result<(), &'static str> {
        match self.state {
            OrderState::Pending => { self.state = OrderState::Paid; Ok(()) }
            _ => Err("temporal: payment_confirmed requires state == Pending"),
        }
    }

    /// Temporal invariant: Shipped only reachable from Paid
    pub fn ship(&mut self) -> Result<(), &'static str> {
        match self.state {
            OrderState::Paid => { self.state = OrderState::Shipped; Ok(()) }
            OrderState::Pending => Err("temporal: cannot ship before payment"),
            _ => Err("temporal: invalid transition to Shipped"),
        }
    }

    /// Temporal invariant: Delivered only reachable from Shipped
    pub fn deliver(&mut self) -> Result<(), &'static str> {
        match self.state {
            OrderState::Shipped => { self.state = OrderState::Delivered; Ok(()) }
            _ => Err("temporal: cannot deliver from non-Shipped state"),
        }
    }

    pub fn cancel(&mut self) -> Result<(), &'static str> {
        match self.state {
            OrderState::Pending | OrderState::Paid => {
                self.state = OrderState::Cancelled; Ok(())
            }
            _ => Err("temporal: cannot cancel Shipped or Delivered order"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporal_payment_before_shipment() {
        let mut order = Order::new(1);
        // Correct order: Pending -> Paid -> Shipped
        assert!(order.confirm_payment().is_ok());
        assert!(order.ship().is_ok());
        assert_eq!(order.state, OrderState::Shipped);
    }

    #[test]
    fn temporal_cannot_ship_before_payment() {
        let mut order = Order::new(2);
        // Violation: try to ship from Pending
        let result = order.ship();
        assert!(result.is_err(), "temporal: shipment before payment must fail");
    }

    #[test]
    fn temporal_full_lifecycle_sequence() {
        let mut order = Order::new(3);
        assert!(order.confirm_payment().is_ok());
        assert!(order.ship().is_ok());
        assert!(order.deliver().is_ok());
        assert_eq!(order.state, OrderState::Delivered);
    }

    #[test]
    fn temporal_liveness_eventually_delivered() {
        // Every valid order that reaches Paid eventually reaches Delivered
        let mut order = Order::new(4);
        order.confirm_payment().unwrap();
        order.ship().unwrap();
        order.deliver().unwrap();
        assert_eq!(order.state, OrderState::Delivered,
            "liveness: paid order must eventually be deliverable");
    }

    #[test]
    fn temporal_cannot_deliver_without_shipping() {
        let mut order = Order::new(5);
        order.confirm_payment().unwrap();
        // Skip shipping — try to deliver directly
        let result = order.deliver();
        assert!(result.is_err(), "temporal: Delivered requires prior Shipped state");
    }
}
