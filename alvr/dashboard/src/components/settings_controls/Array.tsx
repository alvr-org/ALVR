import React from "react"
import { SchemaNode, SessionSettingsNode } from "../../sessionManager"

export function Array(props: {
    schema: SchemaNode[]
    session: SessionSettingsNode[]
    setSession: (session: SessionSettingsNode[]) => void
}): JSX.Element {
    return <>todo array</>
}
