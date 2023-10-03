import { useEffect, useState } from "react"
import { invoke, event } from "@tauri-apps/api";

interface ProgressMessage {
    p_type: String,
    current: number,
    total: number
}



export default function LoginPage() {

    const [greetMessage, setGreetMessage] = useState<String>("");
    const [isLogged, setIsLogged] = useState<boolean>(false);
    const [loginButtonDisabled, setLoginButtonDisabled] = useState<boolean>(false);

    useEffect(() => {
        event.listen('progress', (e) => {
            let v = e.payload as ProgressMessage;
            setGreetMessage("{type: " + v.p_type + ", current: " + v.current + ", total: " + v.total + "}");
            // setGreetMessage(String(e.payload));
        });
        
    }, [])

    async function login () {
        if(!isLogged && !loginButtonDisabled) {
            setLoginButtonDisabled(true);
            invoke("login", {}).then(value => {
                setGreetMessage(String(value));
                setIsLogged(true);
            }).catch(err => {
                setGreetMessage("Error: " + err)
                setLoginButtonDisabled(false);
            })
        }
    }

    async function download() {
        if(isLogged) {
            invoke("download", {}).then(value => {
                setGreetMessage(String(value))
                // this.greet_message = value
        }).catch(err => {
            console.log("An error occured")
            setGreetMessage("Error: " + err)
        })
      }
    }

    return (
    <>
        <h1>Welcome to Tauri!</h1>
        <div className="row">
            <div>
                <button id="greet-button" type="button" onClick={login}>Login to minecraft</button>
                <button id="download-button" className={!isLogged ? "hide" : ""} onClick={download} v-on:click="download">Download game</button>
            </div>
        </div>

        <p id="greet-msg">{ greetMessage }</p>
    </>
    )

}