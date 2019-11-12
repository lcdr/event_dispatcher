/*!
	This crate provides a simple but powerful sync event dispatcher.

	### Type setup

	Create a type with the data you want to pass to event listeners.

	Implement [`Listener`] for your event type on your listener types.

	Create a dispatcher type using the [`dispatcher!`] macro.

	### Usage

	Create a new [dispatcher].

	[Register] your listeners with it.

	[Dispatch] events to the listeners.

	### Examples

	#### Using an owned listener:

	```rust
	use event_dispatcher::{dispatcher, DispatcherCommand, Listener};

	struct Event(i32);

	dispatcher!(Dispatcher<Event, ()>);

	fn main() {
		let mut dispatcher = Dispatcher::default();

		let closure = Box::new(
			move |event: &Event, event_mut: &mut ()| -> Option<DispatcherCommand> {
				println!("Got event with value {}", event.0);
				None
			}
		);

		dispatcher.add(closure);
		dispatcher.dispatch(&Event(42), &mut ());
	}
	```

	#### Using a weakly referenced listener:

	```rust
	use std::cell::RefCell;
	use std::rc::Rc;
	use event_dispatcher::{dispatcher, DispatcherCommand, Listener};

	struct Event(i32);

	dispatcher!(Dispatcher<Event, ()>);

	struct ListenerStruct {}

	impl Listener<Event, ()> for ListenerStruct {
		fn on_event(&mut self, event: &Event, event_mut: &mut ()) -> Option<DispatcherCommand> {
			println!("Got event with value {}", event.0);
			None
		}
	}

	fn main() {
		let listener = Rc::new(RefCell::new(ListenerStruct {}));
		let mut dispatcher = Dispatcher::default();
		dispatcher.add(Box::new(Rc::downgrade(&listener)));
		dispatcher.dispatch(&Event(42), &mut ());
	}
	```

	[`Listener`]: trait.Listener.html
	[`dispatcher!`]: macro.dispatcher.html
	[dispatcher]: struct.DispatcherType.html
	[Register]: struct.DispatcherType.html#method.add
	[Dispatch]: struct.DispatcherType.html#method.dispatch
	[`DispatcherType`]: struct.DispatcherType.html
*/
use std::cell::RefCell;
use std::rc::Weak;

#[cfg(test)]
mod tests;

/**
	Special commands [`Listener`]s can return to the dispatcher to influence dispatching.

	[`Listener`]: trait.Listener.html
*/
#[derive(Debug)]
pub enum DispatcherCommand {
	/// Remove your listener from the event dispatcher.
	/// It will never be called again.
	StopListening,
	/// Stop dispatching of the current [`dispatch`] call.
	/// Any listeners not yet called won't be called, but may be called again in later dispatches.
	///
	/// [`dispatch`]: struct.DispatcherType.html#method.dispatch
	StopPropagation,
	/// Both `StopListening` and `StopPropagation`.
	StopListeningAndPropagation,
}

/**
	Implement this trait in order to receive dispatched events.

	You will be passed a shared reference to the type `E` and a mutable reference to the type `M`. This allows you to use mutable events when necessary but does not force you to do so. If you don't need one of the types, simply use the empty tuple type `()`.
*/
pub trait Listener<E=(), M=()> {
	/**
		This function will be called once a dispatcher you are registered with has an event to dispatch.
		You can influence the dispatcher with the return value, see [`DispatcherCommand`] for details.

		[`DispatcherCommand`]: enum.DispatcherCommand.html
	*/
	fn on_event(&mut self, event: &E, event_mut: &mut M) -> Option<DispatcherCommand>;
}

/// Allows closures to be used as event listeners.
impl<E, M, F> Listener<E, M> for F where F: FnMut(&E, &mut M) -> Option<DispatcherCommand> {
	fn on_event(&mut self, event: &E, event_mut: &mut M) -> Option<DispatcherCommand> {
		(self)(event, event_mut)
	}
}

/// Allows weak references to event listeners to be used as event listeners themselves.
impl<E, M, L: Listener<E, M>> Listener<E, M> for Weak<RefCell<L>> {
	fn on_event(&mut self, event: &E, event_mut: &mut M) -> Option<DispatcherCommand> {
		if let Some(listener_rc) = self.upgrade() {
			let mut listener = listener_rc.borrow_mut();
			listener.on_event(event, event_mut)
		} else {
			Some(DispatcherCommand::StopListening)
		}
	}
}

/**
	Macro to create a dispatcher type specialized to event types.

	This unfortunately can't be done using generics because generics don't support referencing the lifetimes of type parameters, which is necessary to properly define the higher rank trait bounds on listeners. This macro therefore makes it possible to use event types which hold references.

	Call using the name you want to call your dispatcher, the type of event references, the type of mutable event references, and any lifetimes that the event types include.

	See [`DispatcherType`] for documentation on the created type.

	[`DispatcherType`]: struct.DispatcherType.html

	# Examples

	```rust
	use event_dispatcher::{dispatcher, DispatcherCommand, Listener};

	struct Event<'a, 'b> {
		a: &'a bool,
		b: &'b mut bool,
	}

	dispatcher!(MyDispatcher<u32, ()>);
	dispatcher!(MyAdvancedDispatcher<Event<'a, 'b>, Event<'c, 'd>>, 'a, 'b, 'c, 'd);

	// angle brackets not needed afterwards
	let dispatcher_instance = MyDispatcher::default();
	let adv_disp_instance = MyAdvancedDispatcher::default();
	```
*/
#[macro_export]
macro_rules! dispatcher {
	($disp_name:ident<$event:ty, $event_mut:ty>$(, $lifetime:tt)*) => {
		/**
			Docs-only metavariable: Use the [`dispatcher!`] macro to create this type in your code.

			Routes events to registered listeners.

			Allows listeners to be registered using [`add`], and events to be dispatched to those listeners using [`dispatch`].

			[`dispatcher!`]: macro.dispatcher.html
			[`add`]: struct.DispatcherType.html#method.add
			[`dispatch`]: struct.DispatcherType.html#method.dispatch
		*/
		struct $disp_name {
			listeners: Vec<Box<dyn for<$($lifetime,)*> Listener<$event, $event_mut>>>,
		}

		impl $disp_name {
			/**
				Adds a listener to listen for an event. The listener will be called when [`dispatch`] is called.

				[`dispatch`]: struct.DispatcherType.html#method.dispatch
			*/
			pub fn add(&mut self, listener: Box<dyn for<$($lifetime,)*> Listener<$event, $event_mut>>) {
				self.listeners.push(listener);
			}

			/**
				Calls all registered [`Listener`]s via their implemented [`on_event`] method.
				Listeners can influence the dispatcher with the return value, see [`DispatcherCommand`] for details.

				[`Listener`]: trait.Listener.html
				[`on_event`]: trait.Listener.html#tymethod.on_event
				[`DispatcherCommand`]: enum.DispatcherCommand.html
			*/
			pub fn dispatch<$($lifetime,)*>(&mut self, event:&$event, event_mut: &mut $event_mut) {
				let mut i = 0;
				while i < self.listeners.len() {
					let res = self.listeners[i].on_event(event, event_mut);
					match res {
						None => i += 1,
						Some(DispatcherCommand::StopListening) => {
							self.listeners.swap_remove(i);
						}
						Some(DispatcherCommand::StopPropagation) => {
							break;
						}
						Some(DispatcherCommand::StopListeningAndPropagation) => {
							self.listeners.swap_remove(i);
							break;
						}
					}
				}
			}
		}

		impl Default for $disp_name {
			fn default() -> Self {
				Self {
					listeners: vec![],
				}
			}
		}
	}
}
/*
// todo: use this when it is stable
//#[cfg(rustdoc)]
/// Docs-only metavariable: This is the first argument to the [`dispatcher!`] macro.
///
/// [`dispatcher!`]: macro.dispatcher.html
struct EventType;
/// Docs-only metavariable: This is the second argument to the [`dispatcher!`] macro.
///
/// [`dispatcher!`]: macro.dispatcher.html
struct EventMutType;
dispatcher!(DispatcherType<EventType, EventMutType>);
*/
