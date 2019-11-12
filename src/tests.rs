use std::cell::RefCell;
use std::rc::Rc;

use super::{dispatcher, DispatcherCommand, Listener};

dispatcher!(Dispatcher<(), u32>);

struct EventListener {
	ret: Option<DispatcherCommand>
}

impl Listener<(), u32> for EventListener {
	fn on_event(&mut self, _: &(), event_mut: &mut u32) -> Option<DispatcherCommand> {
		*event_mut += 1;
		match &self.ret {
			None => None,
			Some(DispatcherCommand::StopListening) => Some(DispatcherCommand::StopListening),
			Some(DispatcherCommand::StopPropagation) => Some(DispatcherCommand::StopPropagation),
			Some(DispatcherCommand::StopListeningAndPropagation) => Some(DispatcherCommand::StopListeningAndPropagation),
		}
	}
}

/// Tests that an event listener will be called if `dispatch` is called.
#[test]
fn owned() {
	let mut dispatcher = Dispatcher::default();
	let listener = EventListener { ret: None };
	dispatcher.add(Box::new(listener));
	let mut uses = 0;
	dispatcher.dispatch(&(), &mut uses);
	assert_eq!(uses, 1);
}

/// Tests that closures are also accepted as event listeners.
#[test]
fn closure() {
	let mut dispatcher = Dispatcher::default();
	let closure = Box::new(move |_: &(), event_mut: &mut u32| {
		*event_mut += 1;
		None
	});
	dispatcher.add(closure);
	let mut uses = 0;
	dispatcher.dispatch(&(), &mut uses);
	assert_eq!(uses, 1);
}

/// Tests that a weakrefs are also accepted as event listeners.
#[test]
fn weakref() {
	let mut dispatcher = Dispatcher::default();
	let listener = Rc::new(RefCell::new(EventListener { ret: None }));
	dispatcher.add(Box::new(Rc::downgrade(&listener)));
	let mut uses = 0;
	dispatcher.dispatch(&(), &mut uses);
	assert_eq!(uses, 1);
}

/// Tests that expired weakrefs are deleted if a dispatch to them is attempted.
#[test]
fn expired_weakref() {
	let mut dispatcher = Dispatcher::default();
	let listener = Rc::new(RefCell::new(EventListener { ret: None }));
	dispatcher.add(Box::new(Rc::downgrade(&listener)));
	drop(listener);
	assert!(!dispatcher.listeners.is_empty());
	dispatcher.dispatch(&(), &mut 0);
	assert!(dispatcher.listeners.is_empty());
}

/// Tests that multiple listeners will be called.
#[test]
fn propagation() {
	let mut dispatcher = Dispatcher::default();
	let listener_a = EventListener { ret: None };
	let listener_b = EventListener { ret: None };
	dispatcher.add(Box::new(listener_a));
	dispatcher.add(Box::new(listener_b));
	let mut uses = 0;
	dispatcher.dispatch(&(), &mut uses);
	assert_eq!(uses, 2);
}

/// Tests that a listener will not be called after it has requested to stop listening.
#[test]
fn stop_listening() {
	let mut dispatcher = Dispatcher::default();
	let listener = EventListener { ret: Some(DispatcherCommand::StopListening) };
	dispatcher.add(Box::new(listener));
	let mut uses = 0;
	dispatcher.dispatch(&(), &mut uses);
	assert_eq!(uses, 1);
	dispatcher.dispatch(&(), &mut uses);
	assert_eq!(uses, 1);
}

/// Tests that an event will not be dispatched to other listeners if a previous listener has requested to stop propagation.
#[test]
fn stop_propagation() {
	let mut dispatcher = Dispatcher::default();
	let listener_a = EventListener { ret: Some(DispatcherCommand::StopPropagation) };
	let listener_b = EventListener { ret: Some(DispatcherCommand::StopPropagation) };
	dispatcher.add(Box::new(listener_a));
	dispatcher.add(Box::new(listener_b));
	let mut uses = 0;
	dispatcher.dispatch(&(), &mut uses);
	assert_eq!(uses, 1);
	dispatcher.dispatch(&(), &mut uses);
	assert_eq!(uses, 2);
}

/// Tests that both the listener is removed and propagation is stopped if both are requested.
#[test]
fn stop_listening_and_propagation() {
	dispatcher!(Dispatcher<(), ()>, 'a, 'b);

	struct EventListener {
		uses: u32
	}

	impl Listener for EventListener {
		fn on_event(&mut self, _event: &(), _: &mut ()) -> Option<DispatcherCommand> {
			self.uses += 1;
			Some(DispatcherCommand::StopListeningAndPropagation)
		}
	}

	let mut dispatcher = Dispatcher::default();

	let listener_a = Rc::new(RefCell::new(EventListener { uses: 0 }));
	let listener_b = Rc::new(RefCell::new(EventListener { uses: 0 }));
	dispatcher.add(Box::new(Rc::downgrade(&listener_a)));
	dispatcher.add(Box::new(Rc::downgrade(&listener_b)));

	dispatcher.dispatch(&(), &mut ());
	assert_eq!(listener_a.try_borrow_mut().unwrap().uses, 1);
	assert_eq!(listener_b.try_borrow_mut().unwrap().uses, 0);

	dispatcher.dispatch(&(), &mut ());
	assert_eq!(listener_a.try_borrow_mut().unwrap().uses, 1);
	assert_eq!(listener_b.try_borrow_mut().unwrap().uses, 1);

	dispatcher.dispatch(&(), &mut ());
	assert_eq!(listener_a.try_borrow_mut().unwrap().uses, 1);
	assert_eq!(listener_b.try_borrow_mut().unwrap().uses, 1);
}

/// Tests that event types parametric over lifetimes are accepted.
#[test]
fn lifetimes() {
	struct Event<'a, 'b> {
		a: &'a bool,
		b: &'b mut bool,
	}

	dispatcher!(Dispatcher<Event<'a, 'b>, Event<'c, 'd>>, 'a, 'b, 'c, 'd);

	struct EventListener;

	impl Listener<Event<'_, '_>, Event<'_, '_>> for EventListener {
		fn on_event(&mut self, event: &Event<'_, '_>, event_mut: &mut Event<'_, '_>) -> Option<DispatcherCommand> {
			*event_mut.b = *event.a;
			None
		}
	}

	let mut dispatcher = Dispatcher::default();
	let listener = Rc::new(RefCell::new(EventListener));
	dispatcher.add(Box::new(Rc::downgrade(&listener)));
	let event = Event { a: &true, b: &mut false };
	let mut event_mut = Event { a: &false, b: &mut false };
	dispatcher.dispatch(&event, &mut event_mut);
	assert_eq!(*event_mut.b, true);
}
