import React, { useState } from "react"
import {
    Session,
    SessionContext,
    SessionSettingsChoice,
    subscribeToSession,
} from "./sessionManager"
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

// Default theme. The default theme can be overridden with "dark" or "compact", but not vice versa
import "antd/dist/antd.css"

interface SessionData {
    session: Session
    locale: string
    layout: {
        direction: "ltr" | "rtl"
        componentSize: "small" | "middle" | "large"
    }
}

function MenuEntries({
    isMobile,
    onClick,
}: {
    isMobile: boolean
    onClick: (selected: string) => void
}): JSX.Element {
    const theme = isMobile ? "light" : "dark"
    const languagePosition = !isMobile ? "absolute" : undefined
    const languagebottom = !isMobile ? 0 : undefined

    function handleMenuEntryClick({ key }: { key: React.Key }) {
        onClick(key as string)
    }

    return (
        <>
            <Menu theme={theme} defaultSelectedKeys={["clients"]} onClick={handleMenuEntryClick}>
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
            <Menu
                theme={theme}
                selectable={false}
                style={{ position: languagePosition, bottom: languagebottom }}
                onClick={handleMenuEntryClick}
            >
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
    const [title, setTitle] = useState("Clients")

    function handleMenuEntryClick(selection: string) {
        props.selectionHandler(selection)
        if (selection !== "language") {
            setTitle(selection)
            setDrawerOpen(false)
        }
    }

    return (
        <>
            <Drawer
                visible={drawerOpen}
                closable={false}
                placement="left"
                onClose={() => setDrawerOpen(false)}
            >
                <MenuEntries isMobile onClick={handleMenuEntryClick} />
            </Drawer>
            <Layout.Header style={{ padding: 0 }}>
                <Menu selectable={false} onClick={() => setDrawerOpen(true)} mode="horizontal">
                    <Menu.Item>
                        <MenuOutlined style={{ fontSize: "18px" }} />
                        <Typography.Text style={{ fontSize: "20px" }} strong>
                            {title}
                        </Typography.Text>
                    </Menu.Item>
                </Menu>
            </Layout.Header>
        </>
    )
}

export function Dashboard(props: { initialSession: Session }): JSX.Element {
    let themeKey = (props.initialSession.session_settings["extra"][
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
        // Already imported on top
    }

    function getSessionData(session: Session): SessionData {
        const locale = session.session_settings["extra"]["locale"] as string

        const directionString = (session.session_settings["extra"][
            "layout_direction"
        ] as SessionSettingsChoice).variant
        const direction = directionString === "LeftToRight" ? "ltr" : "rtl"

        const componentSizeString = (session.session_settings["extra"][
            "layout_density"
        ] as SessionSettingsChoice).variant
        const componentSize = componentSizeString.toLowerCase() as "small" | "middle" | "large"

        return { session, locale, layout: { direction, componentSize } }
    }

    const [sessionData, setSessionData] = useState(getSessionData(props.initialSession))

    subscribeToSession(session => setSessionData(getSessionData(session)))

    const { xs } = Grid.useBreakpoint()

    const [localeSelection, setLocaleSelection] = useState(sessionData.locale)
    function changeLocale(modalCloseHandle: () => void) {
        //todo
        localeSelection

        modalCloseHandle()
    }

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
                        <Select defaultValue={sessionData.locale} onChange={setLocaleSelection}>
                            <Select.Option value="">System</Select.Option>
                            <Select.Option value="en">English</Select.Option>
                            <Select.Option value="it">Italiano</Select.Option>
                        </Select>
                    </Row>
                ),
            })
        }
    }

    return (
        <SessionContext.Provider value={sessionData.session}>
            <ConfigProvider {...sessionData.layout}>
                <Layout style={{ minHeight: "100vh" }}>
                    {xs ? (
                        <MobileMenu selectionHandler={selectionHandler} />
                    ) : (
                        <DesktopMenu selectionHandler={selectionHandler} />
                    )}
                    <>todo content</>
                </Layout>
            </ConfigProvider>
        </SessionContext.Provider>
    )
}
