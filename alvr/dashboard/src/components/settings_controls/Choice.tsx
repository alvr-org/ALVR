import { Radio, Select, Space } from "antd"
import React, { useContext } from "react"
import { SchemaChoice, SessionSettingsChoice, SessionSettingsNode } from "../../sessionManager"
import { Trans, TransName, useTrans } from "../../translation"
import { AdvancedContext, SettingContainer, SettingControl } from "../Settings"
import { Reset } from "./Reset"

export function ChoiceControl(props: {
    schema: SchemaChoice
    session: SessionSettingsChoice
    setSession: (session: SessionSettingsChoice) => void
}): JSX.Element {
    const showAdvanced = useContext(AdvancedContext)

    const { name: defaultDisplayName } = useTrans(props.schema.default)

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
                                <TransName subkey={variant} />
                            </Radio.Button>
                        )
                    })}
                </Radio.Group>
            ) : (
                <Select value={props.session.variant} onChange={setVariant}>
                    {props.schema.variants.map(([variant]) => {
                        return (
                            <Select.Option value={variant} key={variant}>
                                <TransName subkey={variant} />
                            </Select.Option>
                        )
                    })}
                </Select>
            )}
            <Reset
                default={props.schema.default}
                display={defaultDisplayName}
                reset={() => setVariant(props.schema.default)}
            />
            {maybeContentSchema && (!maybeContentSchema.advanced || showAdvanced) && (
                <Trans node={props.session.variant}>
                    <SettingControl
                        schema={maybeContentSchema.content}
                        session={props.session[props.session.variant]}
                        setSession={setContent}
                    />
                </Trans>
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
            <Trans node={props.session.variant}>
                <SettingContainer
                    schema={maybeContentSchema.content}
                    session={props.session[props.session.variant]}
                    setSession={setContent}
                />
            </Trans>
        )
    } else {
        return null
    }
}
