import React, { CSSProperties, useState } from "react"
import { ConfigProvider, Drawer, Grid, Layout, Menu, Modal, PageHeader, Row, Select } from "antd"
import {
    ApiOutlined,
    AppstoreAddOutlined,
    GlobalOutlined,
    HddOutlined,
    InfoCircleOutlined,
    LineChartOutlined,
    MenuOutlined,
    SettingOutlined,
    TableOutlined,
} from "@ant-design/icons"
import {
    applySessionSettings,
    SessionSettingsChoice,
    SessionSettingsSection,
    SettingsSchema,
    useSession,
} from "./sessionManager"
import { Clients } from "./components/Clients"
import { Statistics } from "./components/Statistics"
import { Presets } from "./components/Presets"
import { Settings } from "./components/Settings"
import { Installation } from "./components/Installation"
import { Logs } from "./components/Logs"
import { About } from "./components/About"

// Import light theme by default to avoid reflow during loading
import "antd/dist/antd.css"
import { TransName } from "./translation"
import { useTranslation } from "react-i18next"

const INITIAL_SELECTED_TAB = "clients"

function MenuEntries({
    isMobile,
    onClick,
}: {
    isMobile: boolean
    onClick: (selected: string) => void
}): JSX.Element {
    const theme = isMobile ? "light" : "dark"
    const style: CSSProperties = !isMobile ? { position: "absolute", bottom: 0 } : {}

    function handleMenuEntryClick({ key }: { key: React.Key }) {
        onClick(key as string)
    }

    return (
        <>
            <Menu
                theme={theme}
                defaultSelectedKeys={[INITIAL_SELECTED_TAB]}
                onClick={handleMenuEntryClick}
            >
                <Menu.Item key="clients" icon={<ApiOutlined />}>
                    <TransName subkey="clients" />
                </Menu.Item>
                <Menu.Item key="statistics" icon={<LineChartOutlined />}>
                    <TransName subkey="statistics" />
                </Menu.Item>
                <Menu.Item key="presets" icon={<AppstoreAddOutlined rotate={-90} />}>
                    <TransName subkey="presets" />
                </Menu.Item>
                <Menu.Item key="settings" icon={<SettingOutlined />}>
                    <TransName subkey="settings" />
                </Menu.Item>
                <Menu.Item key="installation" icon={<HddOutlined />}>
                    <TransName subkey="installation" />
                </Menu.Item>
                <Menu.Item key="logs" icon={<TableOutlined />}>
                    <TransName subkey="logs" />
                </Menu.Item>
                <Menu.Item key="about" icon={<InfoCircleOutlined />}>
                    <TransName subkey="about" />
                </Menu.Item>
            </Menu>
            <Menu theme={theme} selectable={false} style={style} onClick={handleMenuEntryClick}>
                <Menu.Item key="language" icon={<GlobalOutlined />}>
                    <TransName subkey="language" />
                </Menu.Item>
            </Menu>
        </>
    )
}

function DesktopMenu(props: { selectionHandler: (selection: string) => void }): JSX.Element {
    return (
        <Layout.Sider collapsed>
            <MenuEntries isMobile={false} onClick={props.selectionHandler} />
        </Layout.Sider>
    )
}

function MobileMenu(props: { selectionHandler: (selection: string) => void }): JSX.Element {
    const [drawerOpen, setDrawerOpen] = useState(false)
    const [title, setTitle] = useState(INITIAL_SELECTED_TAB)

    function handleMenuEntryClick(selection: string) {
        props.selectionHandler(selection)
        if (selection !== "language") {
            setTitle(selection)
            setDrawerOpen(false)
        }
    }

    return (
        <PageHeader
            backIcon={<MenuOutlined />}
            title={<TransName subkey={title} />}
            onBack={() => setDrawerOpen(true)}
        >
            <Drawer
                visible={drawerOpen}
                closable={false}
                placement="left"
                onClose={() => setDrawerOpen(false)}
            >
                <MenuEntries isMobile onClick={handleMenuEntryClick} />
            </Drawer>
        </PageHeader>
    )
}

export function Dashboard({ settingsSchema }: { settingsSchema: SettingsSchema }): JSX.Element {
    const session = useSession()

    const theme = ((session.session_settings["extra"] as SessionSettingsSection)[
        "theme"
    ] as SessionSettingsChoice).variant

    if (
        (theme === "SystemDefault" && window.matchMedia("(prefers-color-scheme: dark)").matches) ||
        theme === "Dark"
    ) {
        import("antd/dist/antd.dark.css")
    } else if (theme === "Compact") {
        import("antd/dist/antd.compact.css")
    } else {
        // Already imported on top
    }

    const extraSettings = session.session_settings["extra"] as SessionSettingsSection

    let locale = extraSettings["locale"] as string

    const directionString = (extraSettings["layout_direction"] as SessionSettingsChoice).variant
    const direction = directionString === "LeftToRight" ? "ltr" : "rtl"

    const componentSizeString = (extraSettings["layout_density"] as SessionSettingsChoice).variant
    const componentSize = componentSizeString.toLowerCase() as "small" | "middle" | "large"

    const { xs } = Grid.useBreakpoint()

    function changeLocale(modalCloseHandle: () => void) {
        extraSettings["locale"] = locale

        applySessionSettings(session.session_settings)

        modalCloseHandle()
    }

    const [selectedTab, setSelectedTab] = useState(INITIAL_SELECTED_TAB)

    function selectionHandler(selection: string) {
        if (selection === "language") {
            Modal.confirm({
                icon: null,
                title: "Select a language",
                width: 250,
                onOk: changeLocale,
                maskClosable: true,
                content: (
                    <Row justify="center">
                        <Select defaultValue={locale} onChange={value => (locale = value)}>
                            <Select.Option value="">System</Select.Option>
                            <Select.Option value="en-US">English</Select.Option>
                            <Select.Option value="it-IT">Italiano</Select.Option>
                        </Select>
                    </Row>
                ),
            })
        } else {
            setSelectedTab(selection)
        }
    }

    return (
        <ConfigProvider {...{ direction, componentSize }}>
            <Layout>
                {xs ? (
                    <MobileMenu selectionHandler={selectionHandler} />
                ) : (
                    <DesktopMenu selectionHandler={selectionHandler} />
                )}
                <Layout.Content style={{ height: "100vh", overflow: "auto" }}>
                    <div hidden={selectedTab != "clients"}>
                        <Clients />
                    </div>
                    <div hidden={selectedTab != "statistics"}>
                        <Statistics />
                    </div>
                    <div hidden={selectedTab != "presets"}>
                        <Presets />
                    </div>
                    <div hidden={selectedTab != "settings"}>
                        <Settings schema={settingsSchema} />
                    </div>
                    <div hidden={selectedTab != "installation"}>
                        <Installation />
                    </div>
                    <div hidden={selectedTab != "logs"}>
                        <Logs />
                    </div>
                    <div hidden={selectedTab != "about"}>
                        <About />
                    </div>
                </Layout.Content>
            </Layout>
        </ConfigProvider>
    )
}
