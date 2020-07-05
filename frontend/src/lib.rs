// TODO: Remove
#![allow(dead_code, unused_variables)]

use std::collections::BTreeMap;

use seed::{prelude::*, *};
use serde::{Deserialize, Serialize};

const LOGIN: &str = "login";
const STORAGE_KEY: &str = "user";
const INDEX: &str = "index.html";

struct Model {
    input_email: String,
    input_password: String,
    stores: BTreeMap<uuid::Uuid, Store>,
    user: Option<User>,
    page: Page,
}

struct Store {}

#[derive(Serialize, Deserialize)]
struct User {}

enum Page {
    Home,
    Login,
    StoreForm,
    AisleForm,
    ProductForm,
    NotFound,
}

impl Page {
    fn init(mut url: Url, user: Option<&User>, orders: &mut impl Orders<Msg>) -> Self {
        let next = url.next_path_part();
        match next {
            None | Some(INDEX) => {
                if let Some(user) = user {
                    log!("have user");
                    Self::Home
                } else {
                    log!("no user");
                    Self::Login
                }
            }
            Some(LOGIN) => Self::Login,
            _ => {
                log!("Unknown path: {:#?}", next);
                Self::NotFound
            }
        }
    }
}

fn init(url: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders.subscribe(Msg::UrlChanged);

    let user = LocalStorage::get(STORAGE_KEY).ok();

    Model {
        input_email: String::new(),
        input_password: String::new(),
        stores: BTreeMap::new(),
        user: None,
        page: Page::init(url, user.as_ref(), orders),
    }
}

enum Msg {
    UrlChanged(subs::UrlChanged),

    CreateAccountClicked,
    LoginClicked,
    LogoutClicked,
    EmailChanged(String),
    PasswordChanged(String),

    CreateStore,
    EditStore,
    DeleteStore,

    CreateAisle,
    EditAisle,
    DeleteAiles,

    CreateProduct,
    EditProduct,
    DeleteProduct,
}

// `update` describes how to handle each `Msg`.
fn update(msg: Msg, model: &mut Model, _: &mut impl Orders<Msg>) {
    match msg {
        Msg::CreateAccountClicked => {}
        Msg::LoginClicked => {}
        Msg::LogoutClicked => {}
        Msg::EmailChanged(email) => model.input_email = email,
        Msg::PasswordChanged(password) => model.input_password = password,
        Msg::CreateStore => {}
        Msg::EditStore => {}
        Msg::DeleteStore => {}
        Msg::CreateAisle => {}
        Msg::EditAisle => {}
        Msg::DeleteAiles => {}
        Msg::CreateProduct => {}
        Msg::EditProduct => {}
        Msg::DeleteProduct => {}
        Msg::UrlChanged(_) => {}
    }
}

// `view` describes what to display.
fn view(model: &Model) -> Node<Msg> {
    match &model.page {
        Page::Home => div!["home"],
        Page::Login => div![form![
            style! {
                St::Display => "flex",
                St::FlexDirection => "column",
            },
            h1![C!["center"], "Efficio"],
            div![C!["center"], "The smart grocery list"],
            div![
                C!["imgcontainer"],
                img![
                    C!["avatar"],
                    attrs! {
                        At::Src => "./logo.png",
                        At::Alt => "Logo",
                    }
                ]
            ],
            div![
                C!["container"],
                b![label![attrs! {At::For => "email"}, "Email"]],
                input![
                    attrs! {
                        At::Name => "email",
                        At::Type => "text",
                        At::Placeholder => "Enter email",
                        At::Required => AtValue::None,
                    },
                    input_ev(Ev::Input, Msg::EmailChanged)
                ],
                b![label![attrs! {At::For => "password"}, "Password"]],
                input![
                    attrs! {
                        At::Name => "password",
                        At::Placeholder => "Enter Password",
                        At::Type => "password",
                        At::Required => AtValue::None,
                    },
                    input_ev(Ev::Input, Msg::PasswordChanged)
                ],
                button![
                    "Login",
                    ev(Ev::Click, |event| {
                        event.prevent_default();
                        Msg::LoginClicked
                    })
                ],
                div![C!["center"], "Or"],
                button![
                    "Create Account",
                    ev(Ev::Click, |event| {
                        event.prevent_default();
                        Msg::CreateAccountClicked
                    })
                ],
            ],
        ]],
        Page::StoreForm => div!["todo"],
        Page::AisleForm => div!["todo"],
        Page::ProductForm => div!["todo"],
        Page::NotFound => div!["404"],
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    // Mount the `app` to the element with the `id` "app".
    App::start("app", init, update, view);
}
