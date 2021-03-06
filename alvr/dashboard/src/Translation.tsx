import XmlParser from "fast-xml-parser"
import React, { Component } from "react"

type TransNode = {
    name: string
    children: TransNode[]
}

class Translation {
    async compile(langCode: string) {
        if (langCode === "en") {
            langCode = "default"
        }

        const xmlContent = await (await fetch(`/nls/${langCode}.xml`)).text()

        const tree = XmlParser.getTraversalObj(xmlContent, { ignoreAttributes: false })

        alert(tree)
    }
}

const TRANSLATION = new Translation()
export default TRANSLATION