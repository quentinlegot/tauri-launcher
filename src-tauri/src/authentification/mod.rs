use std::{fmt, net::TcpListener, sync::Arc};

use rand::{thread_rng, Rng};
use reqwest::{header::{CONTENT_TYPE, CONNECTION, ACCEPT, AUTHORIZATION}, Client};
use serde_json::{Value, json};
use tokio::{sync::mpsc, join};
use urlencoding::encode;
use serde::{Deserialize, Serialize};
use warp::{Filter, http::Response};
use anyhow::{bail, Result, anyhow};

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

pub struct OauthToken {
    client_id: String,
    redirect: String,
    prompt: Arc<Prompt>
}

struct AccessRefreshToken {
    access_token: String,
    refresh_token: String
}

#[derive(Deserialize, Clone, Debug, Serialize)]
struct XboxAuthData {
    token: String,
    uhs: String
}

#[derive(Deserialize, Clone, Debug)]
pub struct ReceivedCode {
    pub code: String,
    pub state: String,
}

#[derive(Deserialize, Debug)]
pub struct GameProfile {
    pub id: String,
    pub name: String,
    pub skins: Vec<Value>,
    pub capes: Vec<Value>,
}

pub struct Authentification;

impl Authentification {

    fn mojang_auth_token(prompt: Prompt, port: u16) -> OauthToken {
        OauthToken {
            client_id: String::from("89db80b6-8a97-4d00-97e8-48b18f377871"),
            redirect: String::from(format!("http://localhost:{}/api/auth/redirect", port)),
            prompt: Arc::new(prompt)
        }
    }
    
    fn create_link(token: &OauthToken, state: &String)-> String {
        format!("https://login.live.com/oauth20_authorize.srf?client_id={}&response_type=code&redirect_uri={}&scope=Xboxlive.signin+Xboxlive.offline_access&prompt={}&state={}", token.client_id, encode(token.redirect.as_str()), token.prompt, state)
    }

    async fn fetch_oauth2_token(prompt: Prompt, app: tauri::AppHandle) -> Result<(ReceivedCode, OauthToken)> {
        let state: String = thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();

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
            bail!("Cannot create port")
        }
        let token_data = Self::mojang_auth_token(prompt, port);
        let link = Self::create_link(&token_data, &state);

        let second_window = tauri::WindowBuilder::new(
            &app,
            "externam",
            tauri::WindowUrl::External(link.parse().unwrap())
        ).build().expect("Failed to build window");
        let received = Self::listen(port_holder.unwrap()).await?;
        second_window.close()?;

        if received.state != state {
            bail!("CSRF check fail")
        }

        Ok((received, token_data))
    }

    // fn create_link_from_prompt(prompt: Prompt) -> String {
    //     Self::create_link(&Self::mojang_auth_token(prompt))
    // }

    async fn fetch_token(oauth_token: ReceivedCode, token_data: OauthToken, reqwest_client: &Client) -> Result<AccessRefreshToken> {
        let request_body = format!("\
            client_id={}\
            &code={}\
            &grant_type=authorization_code\
            &redirect_uri={}", token_data.client_id, oauth_token.code, token_data.redirect);

        let received : Value = reqwest_client
            .post("https://login.live.com/oauth20_token.srf")
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(request_body.into_bytes())
            .send()
            .await?
            .json()
            .await?;

        let token = AccessRefreshToken {
            access_token: String::from(received["access_token"].as_str().unwrap()),
            refresh_token: String::from(received["refresh_token"].as_str().unwrap())
        };

        Ok(token)
    }

    async fn auth_xbox_live(access_refresh_token: AccessRefreshToken, reqwest_client: &Client) -> Result<XboxAuthData> {
        let request_body: Value = json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", access_refresh_token.access_token)
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        });

        let received: Value = reqwest_client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .json(&request_body)
        .send()
        .await?
        .json()
        .await?;
        Ok(XboxAuthData {
            token: String::from(received["Token"].as_str().unwrap()),
            uhs: String::from(received["DisplayClaims"]["xui"][0]["uhs"].as_str().unwrap())
        })
    }

    async fn fetch_xsts_token(xbl_token: &XboxAuthData, reqwest_client: &Client) -> Result<String> {
        let request_body = json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbl_token.token]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        });
        let received = reqwest_client
            .post("https://xsts.auth.xboxlive.com/xsts/authorize")
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .json(&request_body)
            .send()
            .await?;
        if received.status() == 200 {
            let data : Value = received.json().await?;
            Ok(String::from(data["Token"].as_str().unwrap()))
        } else if received.status() == 401 {
            let data : Value = received.json().await?;
            match data["XErr"].as_u64().unwrap() {
                2148916233 => {
                    bail!("Please sign up to xbox")
                },
                2148916235 => {
                    bail!("Xbox Live is unavailable in your country")
                },
                2148916236 | 2148916237 => {
                    bail!("Your account need adult verification, please visit xbox page")
                },
                2148916238 => {
                    bail!("This account is marked as owned by a child and we cannot process until this account is added to a Family by an adult")
                },
                _ => {
                    bail!("An unknow error occured, error code: {}", data["XErr"].as_u64().unwrap())
                }
            }
        } else {
            bail!("xsts return status code {}", received.status())
        }
    }

    async fn minecraft_auth(uhs: &String, xsts: String, reqwest_client: &Client) -> Result<String> {
        let request_body: Value = json!({
            "identityToken": format!("XBL3.0 x={};{}", uhs, xsts)
        });
        let received : Value = reqwest_client
            .post("https://api.minecraftservices.com/authentication/login_with_xbox")
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .json(&request_body)
            .send()
            .await?
            .json()
            .await?;
        Ok(String::from(received["access_token"].as_str().unwrap())) // return jwt
    }

    async fn fetch_game_ownership(mc_token: &String, reqwest_client: &Client) -> Result<bool> {
        let received : Value = reqwest_client
            .get("https://api.minecraftservices.com/entitlements/mcstore")
            .header(AUTHORIZATION, format!("Bearer {}", mc_token))
            .header(ACCEPT, "application/json")
            .send()
            .await?
            .json()
            .await?;
        if received.is_object() {
            let arr = received.as_object().unwrap();
            Ok(arr.len() != 0)
        } else {
            Ok(false)
        }
    }

    async fn fetch_minecraft_profile(mc_token: &String, reqwest_client: &Client) -> Result<GameProfile> {
        let received: Value  = reqwest_client
            .get("https://api.minecraftservices.com/minecraft/profile")
            .header(AUTHORIZATION, format!("Bearer {}", mc_token))
            .header(ACCEPT, "application/json")
            .send()
            .await?
            .json()
            .await?;
        if let Some(val) = received.get("error") {
            bail!(String::from(val.as_str().unwrap()))
        } else {
            let received: GameProfile = match serde_json::from_value(received) {
                Ok(gp) => gp,
                Err(err) => bail!(err),
            };
            Ok(received)
        }
    }

    pub async fn login(prompt: Prompt, app: tauri::AppHandle) -> Result<GameProfile> {
        let reqwest_client = Client::new();
        let oauth_token = Self::fetch_oauth2_token(prompt, app).await?;
        let access_refresh_token = Self::fetch_token(oauth_token.0, oauth_token.1, &reqwest_client).await?;
        let xbox_auth = Self::auth_xbox_live(access_refresh_token, &reqwest_client).await?;
        let xsts = Self::fetch_xsts_token(&xbox_auth, &reqwest_client).await?;
        let mc_token = Self::minecraft_auth(&xbox_auth.uhs, xsts, &reqwest_client).await?;
        let (is_mc_owner, profile) = join!(Self::fetch_game_ownership(&mc_token, &reqwest_client), Self::fetch_minecraft_profile(&mc_token, &reqwest_client));
        match is_mc_owner {
            Ok(is_mc_owner) => {
                if is_mc_owner || profile.is_ok() /* game pass owner if have a game profile but isn't a owner */ {
                    profile
                } else {
                    bail!("Not a owner of a minecraft copy")
                }
            },
            Err(err) => {
                bail!(err)
            }
        }
    }

    async fn listen(port_holder: TcpListener) -> Result<ReceivedCode> {
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
                        if lang.starts_with("fr_") { // also include canadian, belgium, etc. french
                            message = "Vous pouvez maintenant fermer l'onglet!";
                            break;
                        }
                        else {
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
                } else {
                    Err(warp::reject())
                }
            }
        });
        let port = port_holder.local_addr()?.port();
        drop(port_holder);
        let server = warp::serve(route).bind(([127, 0, 0, 1], port));

        tokio::select! {
            _ = server => bail!("Serve went down unexpectedly!"),
            r = rx.recv() => r.ok_or(anyhow!("Can't receive code!")),
            _ = async {
                tokio::time::sleep(tokio::time::Duration::from_secs(120)).await;
            } => bail!("Wait for too much time"),
        } 
    }


}
