import { useEffect, useState } from "react"
import { invoke, event } from "@tauri-apps/api";
import { AltarikManifest } from "../models/manifest/AltarikManifest";
import AltarikManifestComponent from "./AltarikManifestComponent";

interface ProgressMessage {
    p_type: String,
    current: number,
    total: number
}



export default function LoginPage() {

    const [greetMessage, setGreetMessage] = useState<String>("");
    const [isLogged, setIsLogged] = useState<boolean>(false);
    const [loginButtonDisabled, setLoginButtonDisabled] = useState<boolean>(false);
    const [altarikManifest, setAltarikManifest] = useState<AltarikManifest>();
    const [selectedChapter, setSelectChapter] = useState<number>(-1);

    useEffect(() => {
        event.listen('progress', (e) => {
            let v = e.payload as ProgressMessage;
            setGreetMessage("{type: " + v.p_type + ", current: " + v.current + ", total: " + v.total + "}");
            // setGreetMessage(String(e.payload));
        });
    }, [])

    useEffect(() => {
        if(isLogged) {
            invoke('load_altarik_manifest', {}).then(val => {
                setAltarikManifest(val as AltarikManifest)
            }).catch(err => {
                setGreetMessage("Cannot load altarik manifest: " + err)
            })
        } else {
            setAltarikManifest(undefined)
        }
    }, [isLogged])

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
            if(selectedChapter !== -1 && altarikManifest !== undefined) {
                invoke("download", { selectedChapter: selectedChapter }).then(value => {
                    setGreetMessage(String(value))
                }).catch(err => {
                    console.log("An error occured")
                    setGreetMessage("Error: " + err)
                })
            } else {
                setGreetMessage("Please select a chapter first")
            }
      }
    }

    async function select_chapter(key: number) {
        setSelectChapter(key)
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

        <hr />
        <AltarikManifestComponent manifest={altarikManifest} selectedChapter={selectedChapter} onClickFunction={select_chapter} />
    </>
    )

}