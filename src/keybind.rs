use iced_futures::futures;
use inputbot::{handle_input_events, KeybdKey};

pub struct Keybind {}

impl<H, I> iced_futures::subscription::Recipe<H, I> for Keybind
where
    H: std::hash::Hasher,
{
    type Output = Event;

    fn hash(&self, state: &mut H) {
        use std::hash::Hash;

        std::any::TypeId::of::<Self>().hash(state)
    }

    fn stream(
        self: Box<Self>,
        _input: futures::stream::BoxStream<'static, I>,
    ) -> futures::stream::BoxStream<'static, Self::Output> {
        let (send, recv) = futures::channel::mpsc::unbounded();

        std::thread::spawn(|| {
            KeybdKey::OtherKey(192).bind(move || {
                let _ = send.unbounded_send(Event::Keypress);
            });

            handle_input_events();
        });

        Box::pin(recv)
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Keypress,
}
