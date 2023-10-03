import { AltarikManifest } from "../models/manifest/AltarikManifest";
import ChapterList from "./ChapterList";

interface Props {
    manifest: AltarikManifest | undefined,
    selectedChapter: number,
    onClickFunction: Function
}

export default function AltarikManifestComponent({manifest, selectedChapter, onClickFunction} : Props) {

    return (
        <>
            {manifest != undefined ? <ChapterList chapters={manifest.chapters} selectedChapter={selectedChapter} onClickFunction={onClickFunction} /> : <></>}
        </>
    )

}