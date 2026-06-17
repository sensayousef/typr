use std::sync::{Arc, Mutex};

use super::notifier::StateNotifier;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum RecordingState {
    Ready,
    Recording,
    Transcribing,
}

/// RAII guard that resets state to `Ready` and notifies observers on drop.
/// Created when entering `Transcribing`; guarantees reset on every exit path,
/// including early returns via `?`.
pub struct TranscribeGuard {
    state: Arc<Mutex<RecordingState>>,
    notifier: Arc<dyn StateNotifier>,
}

impl TranscribeGuard {
    pub fn new(state: Arc<Mutex<RecordingState>>, notifier: Arc<dyn StateNotifier>) -> Self {
        Self { state, notifier }
    }
}

impl Drop for TranscribeGuard {
    fn drop(&mut self) {
        let mut s = self.state.lock().unwrap();
        *s = RecordingState::Ready;
        drop(s); // release lock before notifying to avoid holding it during emit
        self.notifier.notify(&RecordingState::Ready);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct MockNotifier {
        called: Arc<AtomicBool>,
    }

    impl StateNotifier for MockNotifier {
        fn notify(&self, _state: &RecordingState) {
            self.called.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn guard_resets_state_to_ready_on_drop() {
        let state = Arc::new(Mutex::new(RecordingState::Transcribing));
        let called = Arc::new(AtomicBool::new(false));
        let notifier = Arc::new(MockNotifier { called: called.clone() });

        {
            let _guard = TranscribeGuard::new(state.clone(), notifier);
            assert_eq!(*state.lock().unwrap(), RecordingState::Transcribing);
        }

        assert_eq!(*state.lock().unwrap(), RecordingState::Ready);
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn guard_notifies_on_simulated_error_exit() {
        let state = Arc::new(Mutex::new(RecordingState::Transcribing));
        let called = Arc::new(AtomicBool::new(false));
        let notifier = Arc::new(MockNotifier { called: called.clone() });

        let result: Result<(), &str> = (|| {
            let _guard = TranscribeGuard::new(state.clone(), notifier);
            Err("simulated transcription failure")?;
            Ok(())
        })();

        assert!(result.is_err());
        assert_eq!(*state.lock().unwrap(), RecordingState::Ready);
        assert!(called.load(Ordering::SeqCst));
    }
}
