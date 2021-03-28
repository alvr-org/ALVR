import React from "react"
import { SchemaText } from "../../sessionManager"

export function Text(props: {
    schema: SchemaText
    session: string
    setSession: (session: string) => void
}): JSX.Element {
    return <>&quot;{props.session}&quot;</>
}
