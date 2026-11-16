import SwiftSyntax

enum SimplifiedType: Equatable {
    case anyLike(String)
    case anyError
    case other

    static func of(_ type: TypeSyntax) -> SimplifiedType {
        switch Syntax(type).as(SyntaxEnum.self) {
        case .identifierType(let identifier):
            if let any = anyName(named: identifier.name.text) {
                return .anyLike(any)
            }
            if identifier.name.text == "Error" {
                return .anyError
            }
            return .other
        case .memberType(let member):
            let base = member.baseType.as(IdentifierTypeSyntax.self)?.name.text
            guard base == nil || base == "Foundation" || base == "Swift" else {
                return .other
            }
            if let any = anyName(named: member.name.text) {
                return .anyLike(any)
            }
            return member.name.text == "Error" ? .anyError : .other
        case .optionalType(let optional):
            return SimplifiedType.of(optional.wrappedType)
        case .implicitlyUnwrappedOptionalType(let optional):
            return SimplifiedType.of(optional.wrappedType)
        case .tupleType(let tuple):
            guard tuple.elements.count == 1, let only = tuple.elements.first else {
                return .other
            }
            return SimplifiedType.of(only.type)
        case .someOrAnyType(let someOrAny):
            let inner = SimplifiedType.of(someOrAny.constraint)
            return someOrAny.someOrAnySpecifier.tokenKind == .keyword(.any) ? inner : .other
        default:
            return .other
        }
    }

    private static func anyName(named name: String) -> String? {
        (name == "Any" || name == "AnyObject") ? name : nil
    }

    var isAnyLike: Bool {
        if case .anyLike = self { return true }
        return false
    }
}
