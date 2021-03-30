import React, { CSSProperties, useState } from "react"
import { ConfigProvider, Drawer, Grid, Layout, Menu, Modal, Row, Select, Typography } from "antd"
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
                <Menu.Item key="clients" icon={<ApiOutlined style={{ fontSize: "18px" }} />}>
                    Clients
                </Menu.Item>
                <Menu.Item key="statistics" icon={<LineChartOutlined />}>
                    Statistics
                </Menu.Item>
                <Menu.Item key="presets" icon={<AppstoreAddOutlined rotate={-90} />}>
                    Presets
                </Menu.Item>
                <Menu.Item key="settings" icon={<SettingOutlined />}>
                    Settings
                </Menu.Item>
                <Menu.Item key="installation" icon={<HddOutlined />}>
                    Installation
                </Menu.Item>
                <Menu.Item key="logs" icon={<TableOutlined />}>
                    Logs
                </Menu.Item>
                <Menu.Item key="about" icon={<InfoCircleOutlined />}>
                    About
                </Menu.Item>
            </Menu>
            <Menu theme={theme} selectable={false} style={style} onClick={handleMenuEntryClick}>
                <Menu.Item key="language" icon={<GlobalOutlined />}>
                    Language
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
        <Layout.Header style={{ padding: 0 }}>
            <Drawer
                visible={drawerOpen}
                closable={false}
                placement="left"
                onClose={() => setDrawerOpen(false)}
            >
                <MenuEntries isMobile onClick={handleMenuEntryClick} />
            </Drawer>
            <Menu selectable={false} onClick={() => setDrawerOpen(true)} mode="horizontal">
                <Menu.Item icon={<MenuOutlined style={{ fontSize: "18px" }} />}>
                    <Typography.Text style={{ fontSize: "20px" }} strong>
                        {title}
                    </Typography.Text>
                </Menu.Item>
            </Menu>
        </Layout.Header>
    )
}

export function Dashboard({ settingsSchema }: { settingsSchema: SettingsSchema }): JSX.Element {
    const session = useSession()

    let themeKey = ((session.session_settings["extra"] as SessionSettingsSection)[
        "theme"
    ] as SessionSettingsChoice).variant

    // debug override
    themeKey = "Light"
    // themeKey = "Dark"

    if (
        (themeKey === "SystemDefault" &&
            window.matchMedia("(prefers-color-scheme: dark)").matches) ||
        themeKey === "Dark"
    ) {
        import("antd/dist/antd.dark.css")
        themeKey = "Dark"
    } else if (themeKey === "Compact") {
        import("antd/dist/antd.compact.css")
    } else {
        import("antd/dist/antd.css")
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
                            <Select.Option value="en">English</Select.Option>
                            <Select.Option value="it">Italiano</Select.Option>
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
                <Layout.Content style={{ height: "100vh" }}>
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
