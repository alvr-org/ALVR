import { List } from "antd"
import React, { useContext } from "react"
import {
    SchemaSectionEntryContent,
    SchemaSection,
    SessionSettingsNode,
    SessionSettingsSection,
} from "../../sessionManager"
import { Trans, useTrans } from "../../translation"
import { AdvancedContext, generateSettingsControl } from "../Settings"
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

    let content: JSX.Element | null = null

    switch (props.schemaContent.type) {
        case "Data": {
            if (showAdvanced || !props.schemaContent.content.advanced) {
                content = generateSettingsControl(
                    props.schemaContent.content.content,
                    props.session,
                    props.setContent,
                )
            }
            break
        }
        case "HigherOrder": {
            if (!showAdvanced) {
                content = <HighOrderSetting schema={props.schemaContent.content} />
            }
            break
        }
        case "Placeholder": {
            if (!showAdvanced) {
                switch (props.name) {
                    case "device_dropdown":
                    case "input_device_dropdown":
                    case "output_device_dropdown": {
                        content = <AudioDropdown name={props.name} />
                        break
                    }
                }
            }
            break
        }
    }
    return (
        content && (
            <List.Item>
                {displayName} {content}
            </List.Item>
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
        <List bordered>
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
        </List>
    )
}
