import React from "react"
import { SchemaBoolean } from "../../sessionManager"

export function Boolean(props: {
    schema: SchemaBoolean
    session: boolean
    setSession: (session: boolean) => void
}): JSX.Element {
    return <>todo boolean</>
}
