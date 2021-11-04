mod dashboard;
mod tabs;
mod theme;

use self::dashboard::DashboardEvent;
use alvr_session::{ServerEvent, SessionDesc};
use iced::{
    container, executor,
    futures::{
        channel::mpsc::{self, UnboundedReceiver, UnboundedSender},
        lock::Mutex,
        stream::{self, BoxStream},
        StreamExt,
    },
    window::{self, Position},
    Application, Command, Element, Settings, Subscription, Text, Toggler,
};
use iced_native::{row, subscription::Recipe};
use std::{
    any::TypeId,
    hash::{Hash, Hasher},
    sync::Arc,
};

use super::{LoadSession, RequestHandler};

pub struct EventsRecipe {
    receiver: Arc<Mutex<UnboundedReceiver<ServerEvent>>>,
}

impl<H: Hasher, E> Recipe<H, E> for EventsRecipe {
    type Output = ServerEvent;

    fn hash(&self, state: &mut H) {
        TypeId::of::<Self>().hash(state);
    }

    fn stream(self: Box<Self>, _: BoxStream<E>) -> BoxStream<ServerEvent> {
        let receiver = Arc::clone(&self.receiver);
        Box::pin(stream::unfold((), move |_| {
            let receiver = Arc::clone(&receiver);
            async move { Some((receiver.lock().await.next().await?, ())) }
        }))
    }
}

struct InitData {
    load_session: Box<LoadSession>,
    request_handler: Box<RequestHandler>,
    event_receiver: Arc<Mutex<UnboundedReceiver<ServerEvent>>>,
}

struct Window {
    request_handler: Box<RequestHandler>,
    event_receiver: Arc<Mutex<UnboundedReceiver<ServerEvent>>>,
    dashboard: dashboard::Dashboard,
}

impl Application for Window {
    type Executor = executor::Default;
    type Message = DashboardEvent;
    type Flags = InitData;

    fn new(mut init_data: InitData) -> (Self, Command<DashboardEvent>) {
        (
            Self {
                dashboard: dashboard::Dashboard::new(
                    (init_data.load_session)(),
                    &mut init_data.request_handler,
                ),
                request_handler: init_data.request_handler,
                event_receiver: init_data.event_receiver,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "ALVR Dashboard".into()
    }

    fn update(&mut self, event: DashboardEvent) -> Command<DashboardEvent> {
        self.dashboard.update(event, &mut *self.request_handler);

        Command::none()
    }

    fn view(&mut self) -> Element<DashboardEvent> {
        self.dashboard.view()
    }

    fn subscription(&self) -> Subscription<DashboardEvent> {
        Subscription::from_recipe(EventsRecipe {
            receiver: Arc::clone(&self.event_receiver),
        })
        .map(DashboardEvent::ServerEvent)
    }
}

pub struct Dashboard {
    event_sender: UnboundedSender<ServerEvent>,
    event_receiver: Arc<Mutex<UnboundedReceiver<ServerEvent>>>,
}

impl Dashboard {
    pub fn new() -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded();
        Self {
            event_sender,
            event_receiver: Arc::new(Mutex::new(event_receiver)),
        }
    }

    pub fn run(&self, load_session: Box<LoadSession>, request_handler: Box<RequestHandler>) {
        Window::run(Settings {
            id: None,
            window: window::Settings {
                size: (800, 600),
                position: Position::Centered,
                icon: None, // todo
                ..Default::default()
            },
            flags: InitData {
                load_session,
                request_handler,
                event_receiver: Arc::clone(&self.event_receiver),
            },
            default_font: None,
            default_text_size: 16,
            text_multithreading: false,
            antialiasing: false,
            exit_on_close_request: true,
        })
        .unwrap();
    }

    pub fn report_event(&self, event: ServerEvent) {
        self.event_sender.unbounded_send(event).unwrap();
    }
}
