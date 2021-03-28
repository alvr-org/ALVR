import { Button, Switch as AntdSwitch, Tabs } from "antd"
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
import { Array } from "./settings_controls/Array"
import { Boolean } from "./settings_controls/Boolean"
import { Choice } from "./settings_controls/Choice"
import { Dictionary } from "./settings_controls/Dictionary"
import { Numeric } from "./settings_controls/Numeric"
import { Optional } from "./settings_controls/Optional"
import { Section } from "./settings_controls/Section"
import { Switch } from "./settings_controls/Switch"
import { Text } from "./settings_controls/Text"
import { Vector } from "./settings_controls/Vector"

export const AdvancedContext = React.createContext(false)

export function Settings({ schema }: { schema: SettingsSchema }): JSX.Element {
    const initialTabKey = schema.content[0][0]

    const [advanced, setAdvanced] = useState(false)

    const { session_settings } = useSession()

    function setRootSession(tabName: string, content: SessionSettingsSection) {
        session_settings[tabName] = content

        applySessionSettings(session_settings)
    }

    return (
        <AdvancedContext.Provider value={advanced}>
            <Tabs
                defaultActiveKey={initialTabKey}
                tabBarExtraContent={
                    <div onClick={() => setAdvanced(!advanced)}>
                        <AntdSwitch checked={advanced} />
                        <Button type="link">Advanced</Button>
                    </div>
                }
            >
                {schema.content.map(([tabName, schemaContent]) => (
                    <Tabs.TabPane tab={tabName} key={tabName}>
                        {generateSettingsControls(
                            schemaContent.content.content,
                            session_settings[tabName],
                            session => setRootSession(tabName, session as SessionSettingsSection),
                        )}
                    </Tabs.TabPane>
                ))}
            </Tabs>
        </AdvancedContext.Provider>
    )
}

export function generateSettingsControls(
    schema: SchemaNode,
    session: SessionSettingsNode,
    setSession: (session: SessionSettingsNode) => void,
): JSX.Element {
    switch (schema.type) {
        case "Section":
            return (
                <Section
                    schema={schema.content as SchemaSection}
                    session={session as SessionSettingsSection}
                    {...{ setSession }}
                />
            )
        case "Choice":
            return (
                <Choice
                    schema={schema.content as SchemaChoice}
                    session={session as SessionSettingsChoice}
                    {...{ setSession }}
                />
            )
        case "Optional":
            return (
                <Optional
                    schema={schema.content as SchemaOptional}
                    session={session as SessionSettingsOptional}
                    {...{ setSession }}
                />
            )
        case "Switch":
            return (
                <Switch
                    schema={schema.content as SchemaSwitch}
                    session={session as SessionSettingsSwitch}
                    {...{ setSession }}
                />
            )
        case "Boolean":
            return (
                <Boolean
                    schema={schema.content as SchemaBoolean}
                    session={session as boolean}
                    {...{ setSession }}
                />
            )
        case "Integer":
        case "Float":
            return (
                <Numeric
                    schema={schema.content as SchemaNumeric}
                    session={session as number}
                    {...{ setSession }}
                />
            )
        case "Text":
            return (
                <Text
                    schema={schema.content as SchemaText}
                    session={session as string}
                    {...{ setSession }}
                />
            )
        case "Array":
            return (
                <Array
                    schema={schema.content as SchemaNode[]}
                    session={session as SessionSettingsNode[]}
                    {...{ setSession }}
                />
            )
        case "Vector":
            return (
                <Vector
                    schema={schema.content as SchemaVector}
                    session={session as SessionSettingsVector}
                    {...{ setSession }}
                />
            )
        case "Dictionary":
            return (
                <Dictionary
                    schema={schema.content as SchemaDictionary}
                    session={session as SessionSettingsDictionary}
                    {...{ setSession }}
                />
            )
    }
}
