mod hooks;

use async_trait::async_trait;

#[cfg(feature = "cache")]
use std::rc::Rc;
#[cfg(feature = "cache")]
use yewdux::store::Store;

pub use hooks::*;

pub mod prelude {
    pub use crate::{hooks::*, Request};
    pub use async_trait::async_trait;

    #[cfg(feature = "cache")]
    pub use crate::CachableRequest;
    #[cfg(feature = "cache")]
    pub use std::rc::Rc;
    #[cfg(feature = "cache")]
    pub use yewdux::store::Store;
}

/// The core request trait which has to be implemented for all handler
/// which can be executed through the use api hook.
#[async_trait(?Send)]
pub trait Request: std::fmt::Debug + PartialEq + Clone {
    /// The error which can occur on request failure
    type Error: Clone + std::fmt::Debug + PartialEq + 'static;

    /// The output type of a succesful request
    type Output: Clone + std::fmt::Debug + PartialEq + 'static;

    /// Run the asynchronous operation responsible for fetching or
    /// computing the requested data
    async fn run(&self) -> Result<Self::Output, Self::Error>;

    /// Store or otherwise handle the data of a succesful run of the request
    /// E.g. Store the requested data in a yewdux store
    fn store(_: Self::Output) {}
}

/// A cachable request which may load a cached result from a yewdux store
#[cfg(feature = "cache")]
pub trait CachableRequest: Request {
    /// The yewdux store used to load the cached result
    type Store: Store + PartialEq;

    /// Optionally extract the requested entity from the yewdux store
    fn load(&self, store: Rc<Self::Store>) -> Option<Self::Output>;
}
