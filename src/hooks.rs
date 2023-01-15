#[cfg(feature = "cache")]
use crate::CachableRequest;
use crate::Request;

use yew::prelude::*;
use yew::suspense::SuspensionResult;
#[cfg(feature = "cache")]
use yewdux::prelude::use_store_value;

/// The basic api hook which requests data on mount and preserves its
/// data through out the component lifetime
#[hook]
pub fn use_api<R: Request + 'static>(request: R) -> SuspensionResult<Result<R::Output, R::Error>> {
    let deps = request.clone();
    let result = inner::use_future_with_deps(
        |_| async move {
            let result = request.run().await;

            if let Ok(ref data) = result {
                R::store(data.to_owned());
            }

            result
        },
        deps,
    )?;

    Ok((*result).to_owned())
}

/// The basic api hook which requests data on mount and preserves its
/// data through out the component lifetime
///
/// The handler is emitted every time a request is issued with the regarding result
#[hook]
pub fn use_api_with_handler<R: Request + 'static>(
    request: R,
    handler: Callback<Result<R::Output, R::Error>, ()>,
) -> SuspensionResult<Result<R::Output, R::Error>> {
    let deps = request.clone();
    let result = inner::use_future_with_deps(
        |_| async move {
            let result = request.run().await;

            handler.emit(result.to_owned());

            if let Ok(ref data) = result {
                R::store(data.to_owned());
            }

            result
        },
        deps,
    )?;

    Ok((*result).to_owned())
}

pub struct LazyResponse<R: Request + 'static> {
    pub run: Callback<(), ()>,
    pub data: Option<SuspensionResult<Result<R::Output, R::Error>>>,
}

/// Useful when not wanting to run a request on mount, e.g. for a logout button
/// You may run the request multiple times through multiple emits of the callback
#[hook]
pub fn use_api_lazy<R: Request + 'static>(request: R) -> LazyResponse<R> {
    let deps = request.clone();
    let (run, result) = inner::use_future_callback(
        |_| async move {
            let result = request.run().await;

            if let Ok(ref data) = result {
                R::store(data.to_owned());
            }

            result
        },
        deps,
    );
    let data = result.map(|res| res.map(|res| (*res).clone()));

    LazyResponse { run, data }
}

/// Useful when not wanting to run a request on mount, e.g. for a logout button
/// You may run the request multiple times through multiple emits of the callback
///
/// The handler is emitted every time a request is issued with the regarding result
#[hook]
pub fn use_api_lazy_with_callback<R: Request + 'static>(
    request: R,
    handler: Callback<Result<R::Output, R::Error>, ()>,
) -> LazyResponse<R> {
    let deps = request.clone();
    let (run, result) = inner::use_future_callback(
        |_| async move {
            let result = request.run().await;

            handler.emit(result.to_owned());

            if let Ok(ref data) = result {
                R::store(data.to_owned());
            }

            result
        },
        deps,
    );
    let data = result.map(|res| res.map(|res| (*res).clone()));

    LazyResponse { run, data }
}

/// Use the locally cached data instead of running the api request if possible
#[cfg(feature = "cache")]
#[hook]
pub fn use_api_or_store<R: Request + CachableRequest + 'static>(
    request: R,
) -> SuspensionResult<Result<R::Output, R::Error>> {
    let store = use_store_value::<R::Store>();
    let deps = request.clone();
    let result = inner::use_future_with_deps(
        |_| async move {
            if let Some(cache) = request.load(store) {
                return Ok(cache);
            }

            let result = request.run().await;

            if let Ok(ref data) = result {
                R::store(data.to_owned());
            }

            result
        },
        deps,
    )?;

    Ok((*result).to_owned())
}

/// Use the locally cached data instead of running the api request if possible
/// Only returns a result once the callback was emitted
#[cfg(feature = "cache")]
#[hook]
pub fn use_api_or_store_lazy<R: Request + CachableRequest + 'static>(
    request: R,
) -> LazyResponse<R> {
    let store = use_store_value::<R::Store>();
    let deps = request.clone();
    let (run, result) = inner::use_future_callback(
        |_| async move {
            if let Some(cache) = request.load(store) {
                return Ok(cache);
            }

            let result = request.run().await;

            if let Ok(ref data) = result {
                R::store(data.to_owned());
            }

            result
        },
        deps,
    );

    let data = result.map(|res| res.map(|res| (*res).clone()));

    LazyResponse { run, data }
}

/// from yew@next
mod inner {
    use std::borrow::Borrow;
    use std::cell::Cell;
    use std::fmt;
    use std::future::Future;
    use std::ops::Deref;
    use std::rc::Rc;

    use yew::prelude::*;
    use yew::suspense::{Suspension, SuspensionResult};

    pub struct UseFutureHandle<O> {
        inner: UseStateHandle<Option<O>>,
    }

    impl<O> Deref for UseFutureHandle<O> {
        type Target = O;

        fn deref(&self) -> &Self::Target {
            self.inner.as_ref().unwrap()
        }
    }

    impl<T: fmt::Debug> fmt::Debug for UseFutureHandle<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("UseFutureHandle")
                .field("value", &format!("{:?}", self.inner))
                .finish()
        }
    }

    #[hook]
    pub fn use_future<F, T, O>(init_f: F) -> SuspensionResult<UseFutureHandle<O>>
    where
        F: FnOnce() -> T,
        T: Future<Output = O> + 'static,
        O: 'static,
    {
        use_future_with_deps(move |_| init_f(), ())
    }

    #[hook]
    pub fn use_future_with_deps<F, D, T, O>(f: F, deps: D) -> SuspensionResult<UseFutureHandle<O>>
    where
        F: FnOnce(Rc<D>) -> T,
        T: Future<Output = O> + 'static,
        O: 'static,
        D: PartialEq + 'static,
    {
        let output = use_state(|| None);
        // We only commit a result if it comes from the latest spawned future. Otherwise, this
        // might trigger pointless updates or even override newer state.
        let latest_id = use_state(|| Cell::new(0u32));

        let suspension = {
            let output = output.clone();

            use_memo_base(
                move |deps| {
                    let self_id = latest_id.get().wrapping_add(1);
                    // As long as less than 2**32 futures are in flight wrapping_add is fine
                    (*latest_id).set(self_id);
                    let deps = Rc::new(deps);
                    let task = f(deps.clone());
                    let suspension = Suspension::from_future(async move {
                        let result = task.await;
                        if latest_id.get() == self_id {
                            output.set(Some(result));
                        }
                    });
                    (suspension, deps)
                },
                deps,
            )
        };

        if suspension.resumed() {
            Ok(UseFutureHandle { inner: output })
        } else {
            Err((*suspension).clone())
        }
    }

    #[hook]
    pub fn use_future_callback<F, D, T, O>(
        f: F,
        deps: D,
    ) -> (
        Callback<(), ()>,
        Option<SuspensionResult<UseFutureHandle<O>>>,
    )
    where
        F: FnOnce(Rc<D>) -> T,
        T: Future<Output = O> + 'static,
        O: 'static,
        D: Clone + PartialEq + 'static,
    {
        let execution = use_state(|| false);
        let execute: Callback<(), ()> = {
            let execution = execution.clone();
            use_callback(move |_, _| execution.set(true), ())
        };

        let output = use_state(|| None);
        // We only commit a result if it comes from the latest spawned future. Otherwise, this
        // might trigger pointless updates or even override newer state.
        let latest_id = use_state(|| Cell::new(0u32));

        let suspension = {
            let output = output.clone();

            let deps = (deps, execution.clone());
            use_memo_base(
                move |deps| {
                    if !(*execution) {
                        return (None, deps);
                    }

                    let self_id = latest_id.get().wrapping_add(1);
                    // As long as less than 2**32 futures are in flight wrapping_add is fine
                    (*latest_id).set(self_id);
                    let task = f(Rc::new(deps.0.clone()));
                    let suspension = Suspension::from_future(async move {
                        let result = task.await;

                        if latest_id.get() == self_id {
                            output.set(Some(result));
                        }
                        execution.set(false);
                    });
                    (Some(suspension), (deps.0.to_owned(), deps.1))
                },
                deps,
            )
        };

        if let Some(ref suspension) = *suspension {
            if suspension.resumed() {
                return (execute, Some(Ok(UseFutureHandle { inner: output })));
            } else {
                return (execute, Some(Err(suspension.clone())));
            }
        }

        if output.is_some() {
            return (execute, Some(Ok(UseFutureHandle { inner: output })));
        }

        (execute, None)
    }

    #[hook]
    pub(crate) fn use_memo_base<T, F, D, K>(f: F, deps: D) -> Rc<T>
    where
        T: 'static,
        F: FnOnce(D) -> (T, K),
        K: 'static + Borrow<D>,
        D: PartialEq,
    {
        struct MemoState<T, K> {
            memo_key: K,
            result: Rc<T>,
        }
        let state = use_mut_ref(|| -> Option<MemoState<T, K>> { None });

        let mut state = state.borrow_mut();
        match &*state {
            Some(existing) if existing.memo_key.borrow() != &deps => {
                // Drop old state if it's outdated
                *state = None;
            }
            _ => {}
        };
        let state = state.get_or_insert_with(|| {
            let (result, memo_key) = f(deps);
            let result = Rc::new(result);
            MemoState { result, memo_key }
        });
        state.result.clone()
    }
}
