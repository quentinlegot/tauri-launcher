import { AltarikManifest, Chapter } from "../models/manifest/AltarikManifest"
import ChapterItem from "./ChapterItem"


interface Props {
    chapters: Chapter[],
    selectedChapter: number,
    onClickFunction: Function
}


export default function ChapterList({ chapters, selectedChapter, onClickFunction }: Props) {



    return (
        <div id="chaptersList">
            {
                chapters.map((chapter, key) => (
                    <ChapterItem chapter={chapter} key={key} isSelected={key === selectedChapter} onClickFunction={() => onClickFunction(key)}/>
                ))
            }
        </div>
    )
}