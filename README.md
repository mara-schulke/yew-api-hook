[![crates.io](https://img.shields.io/crates/v/yew-api-hook.svg)](https://crates.io/crates/yew-api-hook)
[![docs](https://docs.rs/yew-api-hook/badge.svg)](https://docs.rs/yew-api-hook)

# Yew API Hook

Use asynchronous api requests in conjunction with yew's suspense feature

## Usage

```rust
#[function_component]
fn App() -> Html {
    html! {
        <Suspense fallback={html! { {"Loading.."} }}>
            <AsyncComponent />
        </Suspense>
    }
}

#[function_component]
fn AsyncComponent() -> HtmlResult {
    let data = use_api(requests::Fetch { id: 0 })?;

    match data {
        Ok(json) => Ok(html! { {format!("{:#?}", json)} }),
        Err(_) => Ok(html! { {"An error occured"} })
    }
}

mod requests {
    use yew_api_hook::prelude::*;

    type ApiResult = anyhow::Result<serde_json::Value>;

    #[derive(Clone, Debug, PartialEq)]
    pub struct Fetch {
        pub id: u64
    }

    #[async_trait(?Send)]
    impl Request for Fetch {
        type Error = anyhow::Error;
        type Output = serde_json::Value;

        async fn run(&self) -> ApiResult {
            // Use your favorite http or whatever implementation
            get(format!("/entity/{}", self.id)).await
        }
    }
}
```
