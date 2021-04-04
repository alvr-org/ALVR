import { Col, Row, Space } from "antd"
import React, { useContext } from "react"
import {
    SchemaSectionEntryContent,
    SchemaSection,
    SessionSettingsNode,
    SessionSettingsSection,
} from "../../sessionManager"
import { Trans, useTrans } from "../../translation"
import { AdvancedContext, SettingContainer, SettingControl } from "../Settings"
import { AudioDropdown } from "./AudioDropdown"
import { HighOrderSetting } from "./HigherOrderSetting"

function SectionField(props: {
    name: string
    schemaContent: SchemaSectionEntryContent
    session: SessionSettingsNode
    setContent: (session: SessionSettingsNode) => void
}): JSX.Element | null {
    const showAdvanced = useContext(AdvancedContext)

    const { name: displayName } = useTrans()

    let control: JSX.Element | null = null
    let container: JSX.Element | null = null

    switch (props.schemaContent.type) {
        case "Data": {
            if (showAdvanced || !props.schemaContent.content.advanced) {
                control = (
                    <SettingControl
                        schema={props.schemaContent.content.content}
                        session={props.session}
                        setSession={props.setContent}
                    />
                )
                container = (
                    <SettingContainer
                        schema={props.schemaContent.content.content}
                        session={props.session}
                        setSession={props.setContent}
                    />
                )
            }
            break
        }
        case "HigherOrder": {
            if (!showAdvanced) {
                control = <HighOrderSetting schema={props.schemaContent.content} />
            }
            break
        }
        case "Placeholder": {
            if (!showAdvanced) {
                switch (props.name) {
                    case "device_dropdown":
                    case "input_device_dropdown":
                    case "output_device_dropdown": {
                        control = <AudioDropdown name={props.name} />
                        break
                    }
                }
            }
            break
        }
    }
    return (
        (control || container) && (
            <>
                <Row>
                    <Col flex="auto">
                        <Space>
                            {displayName} {control}
                        </Space>
                    </Col>
                </Row>
                {container && (
                    <>
                        <Row style={{ height: 8 }} />
                        <Row>
                            <Col flex="32px" />
                            <Col flex="auto">{container}</Col>
                        </Row>
                    </>
                )}
                <Row style={{ height: 8 }} />
            </>
        )
    )
}

export function Section(props: {
    schema: SchemaSection
    session: SessionSettingsSection
    setSession: (session: SessionSettingsSection) => void
}): JSX.Element {
    function setFieldContent(fieldName: string, content: SessionSettingsNode) {
        props.session[fieldName] = content
        props.setSession(props.session)

        if (fieldName === "theme") {
            window.location.reload()
        }
    }

    return (
        <>
            {props.schema.map(([fieldName, schemaContent]) => (
                <Trans node={fieldName} key={fieldName}>
                    <SectionField
                        name={fieldName}
                        {...{ schemaContent }}
                        session={props.session[fieldName]}
                        setContent={c => setFieldContent(fieldName, c)}
                    />
                </Trans>
            ))}
        </>
    )
}
