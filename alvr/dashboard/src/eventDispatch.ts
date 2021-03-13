const listeners: Record<string, (data: unknown) => void> = {}
let websocket: WebSocket | null = null

function resetWebsocket(): void {
    websocket = new WebSocket(`ws://${window.location.host}/events`)

    websocket.onmessage = msgEv => {
        const event: { id: string; data: unknown } = JSON.parse(msgEv.data)

        const maybeCallback = listeners[event.id]
        maybeCallback?.(event.data)
    }

    websocket.onerror = ev => console.error("EventDispatcher error:", ev)

    websocket.onclose = () => {
        console.info("Event websocket closed. Reopening...")
        resetWebsocket()
    }
}

resetWebsocket()

export function subscribeToEvent(id: string, callback: (data: unknown) => void): void {
    listeners[id] = callback
}
