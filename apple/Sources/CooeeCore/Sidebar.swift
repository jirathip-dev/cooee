import Blocks

extension LayoutNode where Place == LayoutPlace {
    /// A resolved nav's sidebar nodes, `top` then `list`, while the sidebar sits as a column (wide) or menu rows (compact).
    public func sidebarRows(on size: SizeClass) -> [Self] {
        let shown: LayoutPlace = size == .compact ? .menu : .column
        let sidebar = slots["side"]?.first { $0.content == .component("sidebar") && $0.place == shown }
        return ["top", "list"].flatMap { sidebar?.slots[$0] ?? [] }
    }

    /// A resolved nav's avatar-menu rows: its `menu`, and on compact the sidebar's nodes after them.
    public func menuRows(on size: SizeClass) -> [Self] {
        let menu = slots["menu"] ?? []
        guard size == .compact else { return menu }
        return menu + sidebarRows(on: .compact)
    }
}
