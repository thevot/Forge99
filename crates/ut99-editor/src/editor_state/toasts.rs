use super::*;

impl EditorState {
    pub fn show_toast(&mut self, message: impl Into<String>, kind: ToastKind) {
        let id = self.next_toast_id;
        self.next_toast_id = self.next_toast_id.wrapping_add(1);
        self.toasts.push(ToastNotification {
            id,
            message: message.into(),
            kind,
            created_at: Instant::now(),
            duration: Duration::from_millis(3500),
        });
        if self.toasts.len() > 6 {
            self.toasts.remove(0);
        }
    }


    pub fn toast_info(&mut self, message: impl Into<String>) {
        self.show_toast(message, ToastKind::Info);
    }


    pub fn toast_success(&mut self, message: impl Into<String>) {
        self.show_toast(message, ToastKind::Success);
    }


    pub fn toast_warning(&mut self, message: impl Into<String>) {
        self.show_toast(message, ToastKind::Warning);
    }


    pub fn toast_error(&mut self, message: impl Into<String>) {
        self.show_toast(message, ToastKind::Error);
    }


    pub fn prune_expired_toasts(&mut self) {
        let now = Instant::now();
        self.toasts.retain(|t| now.duration_since(t.created_at) < t.duration);
    }


    pub fn dismiss_toast(&mut self, id: u64) {
        self.toasts.retain(|t| t.id != id);
    }

}
