import { Button, Col, Row, Space, Switch as AntdSwitch, Tabs } from "antd"
import React, { useState } from "react"
import {
    applySessionSettings,
    SchemaBoolean,
    SchemaChoice,
    SchemaDictionary,
    SchemaNode,
    SchemaNumeric,
    SchemaOptional,
    SchemaSection,
    SchemaSwitch,
    SchemaText,
    SchemaVector,
    SessionSettingsChoice,
    SessionSettingsDictionary,
    SessionSettingsNode,
    SessionSettingsOptional,
    SessionSettingsSection,
    SessionSettingsSwitch,
    SessionSettingsVector,
    SettingsSchema,
    useSession,
} from "../sessionManager"
import { Trans, TransName } from "../translation"
import { Array } from "./settings_controls/Array"
import { Boolean } from "./settings_controls/Boolean"
import { ChoiceContainer, ChoiceControl } from "./settings_controls/Choice"
import { Dictionary } from "./settings_controls/Dictionary"
import { NumericContainer, NumericControl } from "./settings_controls/Numeric"
import { OptionalContainer, OptionalControl } from "./settings_controls/Optional"
import { Section } from "./settings_controls/Section"
import { SwitchContainer, SwitchControl } from "./settings_controls/Switch"
import { Text } from "./settings_controls/Text"
import { Vector } from "./settings_controls/Vector"

export const AdvancedContext = React.createContext(false)

export function Settings({ schema }: { schema: SettingsSchema }): JSX.Element {
    const initialTabKey = schema.content[0][0]

    const [advanced, setAdvanced] = useState(false)

    const { session_settings } = useSession()

    function setTabContent(tabName: string, content: SessionSettingsSection) {
        applySessionSettings({ [tabName]: content })
    }

    return (
        <AdvancedContext.Provider value={advanced}>
            <Row>
                <Col flex="32px" />
                <Col flex="auto">
                    <Tabs
                        defaultActiveKey={initialTabKey}
                        tabBarExtraContent={
                            <div onClick={() => setAdvanced(!advanced)}>
                                <Button type="link">
                                    <Space>
                                        <AntdSwitch checked={advanced} />
                                        <TransName subkey="advanced-mode" />
                                    </Space>
                                </Button>
                            </div>
                        }
                    >
                        {schema.content.map(([tabName, schemaContent]) => (
                            <Tabs.TabPane tab={<TransName subkey={tabName} />} key={tabName}>
                                <Trans node={tabName}>
                                    <SettingContainer
                                        schema={schemaContent.content.content}
                                        session={session_settings[tabName]}
                                        setSession={session =>
                                            setTabContent(
                                                tabName,
                                                session as SessionSettingsSection,
                                            )
                                        }
                                    />
                                </Trans>
                            </Tabs.TabPane>
                        ))}
                    </Tabs>
                </Col>
                <Col flex="32px" />
            </Row>
        </AdvancedContext.Provider>
    )
}

// SettingControl vs SettingContent explanation:
// Each setting entry has a chain of components. Some components are small enough to be rendered
// inline (control), some need to be rendered below (container), some have both a control and a
// container components. The control and container components work independently from each other:
// when the user interacts with them, each components can request a session update that redraws the
// whole settings tree.

export function SettingControl(props: {
    schema: SchemaNode
    session: SessionSettingsNode
    setSession: (session: SessionSettingsNode) => void
}): JSX.Element | null {
    switch (props.schema.type) {
        case "Choice":
            return (
                <ChoiceControl
                    schema={props.schema.content as SchemaChoice}
                    session={props.session as SessionSettingsChoice}
                    setSession={props.setSession}
                />
            )
        case "Optional":
            return (
                <OptionalControl
                    schema={props.schema.content as SchemaOptional}
                    session={props.session as SessionSettingsOptional}
                    setSession={props.setSession}
                />
            )
        case "Switch":
            return (
                <SwitchControl
                    schema={props.schema.content as SchemaSwitch}
                    session={props.session as SessionSettingsSwitch}
                    setSession={props.setSession}
                />
            )
        case "Boolean":
            return (
                <Boolean
                    schema={props.schema.content as SchemaBoolean}
                    session={props.session as boolean}
                    setSession={props.setSession}
                />
            )
        case "Integer":
        case "Float":
            return (
                <NumericControl
                    schema={props.schema.content as SchemaNumeric}
                    session={props.session as number}
                    setSession={props.setSession}
                />
            )
        default:
            return null
    }
}

export function SettingContainer(props: {
    schema: SchemaNode
    session: SessionSettingsNode
    setSession: (session: SessionSettingsNode) => void
}): JSX.Element | null {
    switch (props.schema.type) {
        case "Section":
            return (
                <Section
                    schema={props.schema.content as SchemaSection}
                    session={props.session as SessionSettingsSection}
                    setSession={props.setSession}
                />
            )
        case "Choice":
            return (
                <ChoiceContainer
                    schema={props.schema.content as SchemaChoice}
                    session={props.session as SessionSettingsChoice}
                    setSession={props.setSession}
                />
            )
        case "Optional":
            return (
                <OptionalContainer
                    schema={props.schema.content as SchemaOptional}
                    session={props.session as SessionSettingsOptional}
                    setSession={props.setSession}
                />
            )
        case "Switch":
            return (
                <SwitchContainer
                    schema={props.schema.content as SchemaSwitch}
                    session={props.session as SessionSettingsSwitch}
                    setSession={props.setSession}
                />
            )
        case "Integer":
        case "Float":
            return (
                <NumericContainer
                    schema={props.schema.content as SchemaNumeric}
                    session={props.session as number}
                    setSession={props.setSession}
                />
            )
        case "Text":
            return (
                <Text
                    schema={props.schema.content as SchemaText}
                    session={props.session as string}
                    setSession={props.setSession}
                />
            )
        case "Array":
            return (
                <Array
                    schema={props.schema.content as SchemaNode[]}
                    session={props.session as SessionSettingsNode[]}
                    setSession={props.setSession}
                />
            )
        case "Vector":
            return (
                <Vector
                    schema={props.schema.content as SchemaVector}
                    session={props.session as SessionSettingsVector}
                    setSession={props.setSession}
                />
            )
        case "Dictionary":
            return (
                <Dictionary
                    schema={props.schema.content as SchemaDictionary}
                    session={props.session as SessionSettingsDictionary}
                    setSession={props.setSession}
                />
            )
        default:
            return null
    }
}
