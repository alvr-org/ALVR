import React, { useContext } from "react"
import { SchemaSwitch, SessionSettingsNode, SessionSettingsSwitch } from "../../sessionManager"
import { Switch as AntdSwitch } from "antd"
import { AdvancedContext, generateSettingsControl } from "../Settings"

export function Switch(props: {
    schema: SchemaSwitch
    session: SessionSettingsSwitch
    setSession: (session: SessionSettingsSwitch) => void
}): JSX.Element {
    const showAdvanced = useContext(AdvancedContext)

    function setEnabled(enabled: boolean) {
        props.session.enabled = enabled
        props.setSession(props.session)
    }

    function setContent(content: SessionSettingsNode) {
        props.session.content = content
        props.setSession(props.session)
    }

    return (
        <>
            <AntdSwitch checked={props.session.enabled} onChange={setEnabled} />
            {props.session.enabled &&
                (!props.schema.content_advanced || showAdvanced) &&
                generateSettingsControl(props.schema.content, props.session.content, setContent)}
        </>
    )
}
