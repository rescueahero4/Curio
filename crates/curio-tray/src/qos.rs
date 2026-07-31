//! EcoQoS for the service thread (R-BE-31, ARCH-01 OQ-5).
//!
//! Curio sits in the tray all day doing nothing, and the budget table asks for "no sleep
//! inhibition; EcoQoS on service thread (Win)". EcoQoS asks Windows to schedule a thread
//! on efficiency cores at a lower frequency — measurably better battery for work whose
//! latency nobody is waiting on.
//!
//! Applied to the **service thread only**. The main thread runs the native event loop and
//! stays at default QoS: it handles the tray menu, and a user clicking Pause should not
//! wait behind a throttled scheduler for the sake of a workload that is idle anyway.
//!
//! ## The ControlMask/StateMask gotcha
//!
//! `THREAD_POWER_THROTTLING_STATE` carries two bitmasks, and their combination — not
//! either one alone — selects the mode. This is the part that is easy to get backwards,
//! because the "off" and "let the system decide" cases look similar and behave very
//! differently:
//!
//! | ControlMask | StateMask | Meaning |
//! |---|---|---|
//! | `EXECUTION_SPEED` | `EXECUTION_SPEED` | **throttling ON** — this is EcoQoS |
//! | `EXECUTION_SPEED` | `0` | throttling explicitly OFF |
//! | `0` | `0` | clear the override, let Windows decide |
//!
//! Setting `StateMask` without the matching `ControlMask` bit does nothing at all: the
//! control mask is what says "I am managing this property", and the state mask is only
//! read for the properties it names.

/// Ask Windows to schedule the calling thread as background work.
///
/// Returns whether the request was accepted. A refusal is **not** an error worth failing
/// boot over — the documented fallback is to ship at default QoS (ARCH-07's D0 index), and
/// a machine that declines still runs Curio correctly, just slightly warmer.
pub fn request_eco_qos() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{
            GetCurrentThread, SetThreadInformation, THREAD_POWER_THROTTLING_CURRENT_VERSION,
            THREAD_POWER_THROTTLING_EXECUTION_SPEED, THREAD_POWER_THROTTLING_STATE,
            ThreadPowerThrottling,
        };

        let state = THREAD_POWER_THROTTLING_STATE {
            Version: THREAD_POWER_THROTTLING_CURRENT_VERSION,
            // Both masks carry the same bit: "I am managing execution speed" and "the
            // value I want is throttled". See the table above — one without the other is
            // silently a no-op.
            ControlMask: THREAD_POWER_THROTTLING_EXECUTION_SPEED,
            StateMask: THREAD_POWER_THROTTLING_EXECUTION_SPEED,
        };

        // SAFETY: `state` is a fully initialised value that outlives the call, and its
        // size is what the API expects for `ThreadPowerThrottling`. `GetCurrentThread`
        // returns a pseudo-handle that needs no closing.
        let applied = unsafe {
            SetThreadInformation(
                GetCurrentThread(),
                ThreadPowerThrottling,
                std::ptr::from_ref(&state).cast(),
                u32::try_from(size_of::<THREAD_POWER_THROTTLING_STATE>()).unwrap_or(0),
            )
        };

        if applied == 0 {
            tracing::info!(
                error = %std::io::Error::last_os_error(),
                "EcoQoS was declined; the service thread runs at default QoS"
            );
            return false;
        }
        tracing::debug!("service thread running at EcoQoS");
        true
    }

    #[cfg(not(windows))]
    {
        // macOS has an equivalent in QoS classes, but the budget table asks for this on
        // Windows specifically and SMAppService-era macOS already de-prioritises a
        // background app. Nothing to do rather than something approximate.
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn windows_accepts_the_request() {
        // ARCH-01 OQ-5 is exactly this: does the call succeed with the documented
        // ControlMask/StateMask pairing on Windows 11? A silent no-op would look identical
        // to success from the outside, which is why the return value is checked rather
        // than the call merely being made.
        assert!(
            request_eco_qos(),
            "SetThreadInformation(ThreadPowerThrottling) was refused"
        );
    }

    #[test]
    #[cfg(windows)]
    fn the_request_is_idempotent() {
        // The service thread applies this once at start, but a future restart-in-place
        // would call it again; re-applying the same state must not fail.
        assert!(request_eco_qos());
        assert!(request_eco_qos());
    }

    #[test]
    #[cfg(not(windows))]
    fn other_platforms_decline_rather_than_pretend() {
        assert!(!request_eco_qos());
    }
}
