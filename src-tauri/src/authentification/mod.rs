use std::{fmt, net::TcpListener, io::Result};

use log4rs::Handle;
use reqwest::header::{CONTENT_TYPE, CONNECTION};
use tokio::sync::mpsc;
use urlencoding::encode;
use serde::Deserialize;
use warp::{Filter, http::Response};

pub enum Prompt {
    Login,
    None,
    Consent,
    SelectAccount
}

impl fmt::Display for Prompt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            Prompt::Login => "login",
            Prompt::None => "none",
            Prompt::Consent => "consent",
            Prompt::SelectAccount => "select_account"
        })
    }
}

pub struct Token {
    client_id: String,
    redirect: String,
    prompt: Prompt
}

#[derive(Deserialize, Clone, Debug)]
    struct ReceivedCode {
        pub code: String,
        pub state: String,
    }

pub struct Authentification {
    logger: Handle
}

impl Authentification {

    pub fn new(logger: Handle) -> Self {
        Authentification { logger }
    }

    pub fn mojang_auth_token(prompt: Prompt) -> Token {
        Token {
            client_id: String::from("00000000402b5328"),
            redirect: String::from("https://localhost:PORT/api/auth/redirect"),
            prompt
        }
    }
    
    pub fn create_link(token: Token)-> String {

        format!("https://login.live.com/oauth20_authorize.srf?client_id={}&response_type=code&redirect_uri={}&scope=XboxLive.signin%20offline_access&prompt={}", token.client_id, encode(token.redirect.as_str()), token.prompt)
    }

    pub fn create_link_from_prompt(prompt: Prompt) -> String {
        Self::create_link(Self::mojang_auth_token(prompt))
    }

    pub async fn launch(prompt: Prompt, app: tauri::AppHandle) -> Result<()> {
        // let reqwest_client = ReqwestClient::new();
        let token = Self::mojang_auth_token(prompt);
        let reqwest_client = reqwest::Client::new();
        let mut port_holder = None;
        let mut port = 0;
        for i in 7878..65535 {
            if let Ok(l) = TcpListener::bind(("127.0.0.1", i)) {
                port = l.local_addr()?.port();
                port_holder = Some(l);
                break;
            }
        };
        if port_holder.is_none() {
            Err(())
        }
        let redirect_uri = token.redirect.replace("PORT", &port.to_string());
        let link = Self::create_link_from_prompt(token.prompt);
    
        let second_window = tauri::WindowBuilder::new(
            &app,
            "externam",
            tauri::WindowUrl::External(link.parse().unwrap())
        ).build().expect("Failed to build window");
        let received = Self::listen(port_holder.unwrap()).await?;

        
        Ok(())
    }

    pub async fn listen(port_holder: TcpListener) -> Result<ReceivedCode> {
        let (tx, mut rx) = mpsc::channel::<ReceivedCode>(2);
        let route = warp::query::<ReceivedCode>()
        .and(warp::header::<String>("accept-language"))
        .and_then(move |r: ReceivedCode, accept_lang: String| {
            let tx = tx.clone();
            async move {
                if r.code.is_empty() || r.state.is_empty() {
                    return Err(warp::reject());
                }
                let mut message = "";
                if !accept_lang.is_empty() {
                    let langs = accept_lang.split(",");
                    for lang in langs {
                        if lang.starts_with("fr_FR") {
                            message = "Vous pouvez maintenant fermer l'onglet!";
                            break;
                        }
                        else if lang.starts_with("en") {
                            message = "You can close this tab now!";
                            break;
                        }
                    }
                }
                if message.is_empty() {
                    message = "You can close this tab now!"
                }
                if let Ok(_) = tx.send(r).await {
                    Ok(Response::builder()
                        .header(CONTENT_TYPE, "text/html; charset=UTF-8")
                        .header(CONNECTION, "close")
                        .body(format!("<h1>{}</h1>", message))
                    )
                }
                else {
                    Err(warp::reject())
                }
            }
        });
        Ok(ReceivedCode { code: "".to_string(), state: "".to_string() })
        
    }

    pub async fn xbox_auth(&self, access_token: String) -> () {
        let str = format!(r#"{{
            "Properties": {{
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": "d={}"
            }},
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }}
        "#, access_token);
        let req = reqwest::Client::new();
        let r_xbox_live = req.post("https://user.auth.xboxlive.com/user/authenticate")
            .body(str)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::ACCEPT, "application/json")
            .send().await;
            match r_xbox_live {
                Ok(response) => {
                    let content = response.text().await;
                    match content {
                        Ok(text) => {
                            println!("Sucess: {:?}", text);
                        },
                        Err(err) => {
                            eprintln!("error 2: {:?}", err);
                        }
                    };
                },
                Err(err) => {
                    eprintln!("Error 1: {:?}", err);
                }
            };
    }

}
