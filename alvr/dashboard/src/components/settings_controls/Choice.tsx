import { Radio, Select, Space } from "antd"
import React, { useContext } from "react"
import { SchemaChoice, SessionSettingsChoice, SessionSettingsNode } from "../../sessionManager"
import { AdvancedContext, SettingContainer, SettingControl } from "../Settings"

export function ChoiceControl(props: {
    schema: SchemaChoice
    session: SessionSettingsChoice
    setSession: (session: SessionSettingsChoice) => void
}): JSX.Element {
    const showAdvanced = useContext(AdvancedContext)

    const maybeContentSchema = props.schema.variants.find(
        ([variant]) => variant === props.session.variant,
    )?.[1]

    function setContent(content: SessionSettingsNode) {
        props.session[props.session.variant] = content
        props.setSession(props.session)
    }

    function setVariant(variantName: string) {
        props.session.variant = variantName
        props.setSession(props.session)
    }

    return (
        <Space>
            {props.schema.gui === "ButtonGroup" ? (
                <Radio.Group
                    value={props.session.variant}
                    buttonStyle="solid"
                    onChange={e => setVariant(e.target.value)}
                >
                    {props.schema.variants.map(([variant]) => {
                        return (
                            <Radio.Button value={variant} key={variant}>
                                {variant}
                            </Radio.Button>
                        )
                    })}
                </Radio.Group>
            ) : (
                <Select value={props.session.variant} onChange={setVariant}>
                    {props.schema.variants.map(([variant]) => {
                        return (
                            <Select.Option value={variant} key={variant}>
                                {variant}
                            </Select.Option>
                        )
                    })}
                </Select>
            )}
            {maybeContentSchema && (!maybeContentSchema.advanced || showAdvanced) && (
                <SettingControl
                    schema={maybeContentSchema.content}
                    session={props.session[props.session.variant]}
                    setSession={setContent}
                />
            )}
        </Space>
    )
}

export function ChoiceContainer(props: {
    schema: SchemaChoice
    session: SessionSettingsChoice
    setSession: (session: SessionSettingsChoice) => void
}): JSX.Element | null {
    const showAdvanced = useContext(AdvancedContext)

    const maybeContentSchema = props.schema.variants.find(
        ([variant]) => variant === props.session.variant,
    )?.[1]

    function setContent(content: SessionSettingsNode) {
        props.session[props.session.variant] = content
        props.setSession(props.session)
    }

    if (maybeContentSchema && (!maybeContentSchema.advanced || showAdvanced)) {
        return (
            <SettingContainer
                schema={maybeContentSchema.content}
                session={props.session[props.session.variant]}
                setSession={setContent}
            />
        )
    } else {
        return null
    }
}
