const listeners: Map<string, (data: unknown) => void> = new Map()

function resetWebsocket(): void {
    const websocket = new WebSocket(`ws://${window.location.host}/events`)

    websocket.onmessage = msgEv => {
        const event: { id: string; data: unknown } = JSON.parse(msgEv.data)

        const maybeCallback = listeners.get(event.id)
        maybeCallback && maybeCallback(event.data)
    }

    websocket.onerror = ev => console.error("EventDispatcher error:", ev)

    websocket.onclose = () => {
        console.info("Event websocket closed. Reopening...")
        resetWebsocket()
    }
}

resetWebsocket()

export default function subscribeToEvent(id: string, callback: (data: unknown) => void): void {
    listeners.set(id, callback)
}
