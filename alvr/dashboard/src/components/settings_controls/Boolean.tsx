import React from "react"
import { SchemaBoolean } from "../../sessionManager"
import { Switch } from "antd"

export function Boolean(props: {
    schema: SchemaBoolean
    session: boolean
    setSession: (session: boolean) => void
}): JSX.Element {
    return <Switch checked={props.session} onChange={props.setSession} />
}
