import { title } from "process"
import { Chapter } from "../models/manifest/AltarikManifest"
import { MouseEventHandler } from "react"

interface Props {
    chapter: Chapter,
    isSelected: boolean,
    onClickFunction: MouseEventHandler<HTMLButtonElement>
}

export default function ChapterItem({ chapter, isSelected, onClickFunction } : Props) {

    return (
        <button className={isSelected ? "selected": ""} onClick={onClickFunction}>{chapter.title} -- {chapter.minecraftVersion}</button>
    )
}