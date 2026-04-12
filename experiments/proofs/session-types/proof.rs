// proof.rs — emitted by: loom compile proof.loom
// Theory: Session Types (Honda 1993)
// Each protocol step is a distinct Rust type. Transition methods consume `self`
// (move semantics). Wrong order = wrong type = compile error.

use std::marker::PhantomData;

// ── AuthProtocol typestate ────────────────────────────────────────────────────

pub struct AuthProtocolClientStep0;
pub struct AuthProtocolClientStep1;
pub struct AuthProtocolClientDone;

pub struct AuthProtocolClientChannel<State> {
    _state: PhantomData<State>,
}

impl AuthProtocolClientChannel<AuthProtocolClientStep0> {
    pub fn new() -> Self { Self { _state: PhantomData } }

    /// Step 0: send credentials. Consumes self — Step0 is gone after this call.
    pub fn send(self, _msg: String) -> AuthProtocolClientChannel<AuthProtocolClientStep1> {
        AuthProtocolClientChannel { _state: PhantomData }
    }
}

impl AuthProtocolClientChannel<AuthProtocolClientStep1> {
    /// Step 1: receive token. Consumes self — Step1 is gone after this call.
    pub fn recv(self) -> (AuthProtocolClientChannel<AuthProtocolClientDone>, i64) {
        (AuthProtocolClientChannel { _state: PhantomData }, 42)
    }
}

// ── PaymentProtocol typestate ─────────────────────────────────────────────────

pub struct PaymentProtocolPayerStep0;
pub struct PaymentProtocolPayerStep1;
pub struct PaymentProtocolPayerStep2;
pub struct PaymentProtocolPayerDone;

pub struct PaymentChannel<State> { _state: PhantomData<State> }

impl PaymentChannel<PaymentProtocolPayerStep0> {
    pub fn new() -> Self { Self { _state: PhantomData } }
    pub fn send_amount(self, _amount: f64) -> PaymentChannel<PaymentProtocolPayerStep1> {
        PaymentChannel { _state: PhantomData }
    }
}
impl PaymentChannel<PaymentProtocolPayerStep1> {
    pub fn recv_reference(self) -> (PaymentChannel<PaymentProtocolPayerStep2>, String) {
        (PaymentChannel { _state: PhantomData }, "REF-001".into())
    }
}
impl PaymentChannel<PaymentProtocolPayerStep2> {
    pub fn send_confirmation(self, _code: i64) -> PaymentChannel<PaymentProtocolPayerDone> {
        PaymentChannel { _state: PhantomData }
    }
}

// ── Proof: correct order compiles ────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_auth_order_compiles_and_runs() {
        let ch = AuthProtocolClientChannel::new();
        let ch = ch.send("user:password".to_string());
        let (_ch, token) = ch.recv();
        assert_eq!(token, 42);
    }

    #[test]
    fn correct_payment_order_compiles_and_runs() {
        let ch = PaymentChannel::new();
        let ch = ch.send_amount(99.99);
        let (ch, reference) = ch.recv_reference();
        let _ch = ch.send_confirmation(1234);
        assert!(!reference.is_empty());
    }

    // ── Violation proof (uncomment to see compile error) ─────────────────────
    //
    // #[test]
    // fn wrong_order_recv_before_send() {
    //     let ch = AuthProtocolClientChannel::new();
    //     let (_, _token) = ch.recv(); // ERROR: no method `recv` on Step0
    // }
    //
    // #[test]
    // fn wrong_order_double_send() {
    //     let ch = AuthProtocolClientChannel::new();
    //     let ch = ch.send("first".to_string());
    //     let _ch = ch.send("second".to_string()); // ERROR: ch moved, Step1 has no `send`
    // }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Session types: any valid credential string works in the auth protocol
        #[test]
        fn auth_protocol_valid_sequence_always_succeeds(
            credential in "[a-zA-Z0-9:_@.]{1,100}"
        ) {
            let ch = AuthProtocolClientChannel::new();
            let ch = ch.send(credential);
            let (_ch, token) = ch.recv();
            prop_assert_eq!(token, 42, "auth protocol must return valid token");
        }

        /// Payment protocol: any positive amount completes the sequence
        #[test]
        fn payment_protocol_valid_sequence_always_succeeds(
            amount in 0.01f64..1_000_000.0f64,
            confirmation_code in any::<i64>(),
        ) {
            let ch = PaymentChannel::new();
            let ch = ch.send_amount(amount);
            let (ch, reference) = ch.recv_reference();
            let _ch = ch.send_confirmation(confirmation_code);
            prop_assert!(!reference.is_empty(), "payment protocol must produce reference");
        }
    }
}
