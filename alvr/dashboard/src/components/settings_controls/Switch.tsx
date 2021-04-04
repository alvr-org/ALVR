import React, { useContext } from "react"
import { SchemaSwitch, SessionSettingsNode, SessionSettingsSwitch } from "../../sessionManager"
import { Space, Switch as AntdSwitch } from "antd"
import { AdvancedContext, SettingContainer, SettingControl } from "../Settings"

export function SwitchControl(props: {
    schema: SchemaSwitch
    session: SessionSettingsSwitch
    setSession: (session: SessionSettingsSwitch) => void
}): JSX.Element {
    function setEnabled(enabled: boolean) {
        props.session.enabled = enabled
        props.setSession(props.session)
    }

    function setContent(content: SessionSettingsNode) {
        props.session.content = content
        props.setSession(props.session)
    }

    return (
        <Space>
            <AntdSwitch checked={props.session.enabled} onChange={setEnabled} />
            <SettingControl
                schema={props.schema.content}
                session={props.session.content}
                setSession={setContent}
            />
        </Space>
    )
}

export function SwitchContainer(props: {
    schema: SchemaSwitch
    session: SessionSettingsSwitch
    setSession: (session: SessionSettingsSwitch) => void
}): JSX.Element | null {
    const showAdvanced = useContext(AdvancedContext)

    function setContent(content: SessionSettingsNode) {
        props.session.content = content
        props.setSession(props.session)
    }

    if (props.session.enabled && (!props.schema.content_advanced || showAdvanced)) {
        return (
            <SettingContainer
                schema={props.schema.content}
                session={props.session.content}
                setSession={setContent}
            />
        )
    } else {
        return null
    }
}
