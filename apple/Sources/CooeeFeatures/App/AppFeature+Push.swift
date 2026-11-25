import ComposableArchitecture

extension AppFeature {
    public var body: some ReducerOf<Self> {
        app.onChange(of: Self.showing) { _, thread in show(thread) }
    }
}
