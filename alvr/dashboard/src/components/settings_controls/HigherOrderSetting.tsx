import { Button, Radio, Select, Space, Switch } from "antd"
import React from "react"
import { useTranslation } from "react-i18next"
import {
    applySessionSettings,
    ChoiceHosSchema,
    SchemaHOS,
    SessionSettingsRoot,
    useSession,
} from "../../sessionManager"
import { TransName, useTrans } from "../../translation"
import { Reset } from "./Reset"

function ChoiceHOS(props: {
    schema: ChoiceHosSchema
    isMatching: (value: string) => boolean
    setSession: (value: string) => void
}) {
    const { t } = useTranslation()
    const { name: defaultDisplayName } = useTrans(props.schema.default)

    const value = props.schema.variants.find(props.isMatching)

    let control: JSX.Element
    if (props.schema.gui === "ButtonGroup") {
        control = (
            <Radio.Group
                value={value || ""}
                buttonStyle="solid"
                onChange={e => props.setSession(e.target.value)}
            >
                {props.schema.variants.map(variant => (
                    <Radio.Button value={variant} key={variant}>
                        <TransName subkey={variant} />
                    </Radio.Button>
                ))}
            </Radio.Group>
        )
    } else {
        control = (
            <Select value={value || t("common.choice-custom")} onChange={props.setSession}>
                {props.schema.variants.map(variant => (
                    <Select.Option value={variant} key={variant}>
                        <TransName subkey={variant} />
                    </Select.Option>
                ))}
            </Select>
        )
    }

    return (
        <Space>
            {control}
            <Reset
                default={props.schema.default}
                display={defaultDisplayName}
                reset={() => props.setSession(props.schema.default)}
            />
        </Space>
    )
}

export function HighOrderSetting({ schema }: { schema: SchemaHOS }): JSX.Element {
    const { session_settings } = useSession()

    const { t } = useTranslation()

    function apply(settings: SessionSettingsRoot, input?: string | boolean) {
        for (const modifier of schema.modifiers) {
            // Work around Webpack minification
            const replacedModifier = modifier
                .replaceAll("{settings}", Object.keys({ settings })[0])
                .replaceAll("{input}", Object.keys({ input })[0])

            // Note: modifiers usually need to access the variables "settings" and "input".
            // "modifier" can be any javascript code so this could be dangerous
            eval(replacedModifier)
        }
    }

    function isMatching(value?: string | boolean): boolean {
        const currentSettingsJson = JSON.stringify(session_settings)

        // structural copy
        const settings = JSON.parse(currentSettingsJson) as SessionSettingsRoot

        apply(settings, value)

        // structural equality
        return JSON.stringify(settings) === currentSettingsJson
    }

    function setSession(value?: string | boolean) {
        const session = JSON.parse(JSON.stringify(session_settings)) as SessionSettingsRoot
        apply(session, value)
        applySessionSettings(session)
    }

    switch (schema.data_type.type) {
        case "Choice":
            return <ChoiceHOS schema={schema.data_type.content} {...{ isMatching, setSession }} />
        case "Boolean": {
            const defaultvalue = schema.data_type.content.default
            const value = isMatching(true) || (isMatching(false) ? false : undefined)

            return (
                <Space>
                    {value !== null ? (
                        <Switch checked={value} onChange={value => setSession(value)} />
                    ) : (
                        <Button onClick={() => setSession(defaultvalue)}>
                            <Space>
                                <Switch disabled />
                                {t("common.switch-unset")}
                            </Space>
                        </Button>
                    )}

                    <Reset
                        default={defaultvalue}
                        display={
                            defaultvalue ? t("common.switch-enabled") : t("common.switch-disabled")
                        }
                        reset={() => setSession(defaultvalue)}
                    />
                </Space>
            )
        }
        case "Action": {
            const applied = isMatching()

            return (
                <Button disabled={!applied} onClick={() => setSession()}>
                    {applied ? t("presets.action-applied") : t("presets.action-apply")}
                </Button>
            )
        }
    }
}
