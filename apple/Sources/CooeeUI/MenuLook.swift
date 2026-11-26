import CooeeCore
import CooeeFeatures

extension MenuFeature.State {
    internal func look(dark: Bool, reduceMotion: Bool) -> Look {
        let theme = settings.theme
        let art = server.flatMap { looks.art(theme, on: $0.url) }
        var look = Look(palette: palette(device: dark), style: style(for: theme) ?? .bundled, art: art)
        look.moving = settings.moves(reduceMotion: reduceMotion)
        look.night = settings.night
        look.sign = art != nil && looks.themes[theme]?.sign == true
        look.sprites = settings.sprites(in: theme)
        look.words = looks.themes[theme]?.words ?? [:]
        return look
    }
}
