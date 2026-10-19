import Foundation

/// Conversion d'unités basée sur l'API `Measurement` d'Apple (entièrement locale).
public enum UnitConversion {

    public struct UnitInfo: Identifiable, Hashable, Sendable {
        public var id: String { symbol }
        public let symbol: String
        public let name: String
        let unit: Dimension
        let aliases: [String]

        public static func == (a: UnitInfo, b: UnitInfo) -> Bool { a.symbol == b.symbol }
        public func hash(into h: inout Hasher) { h.combine(symbol) }
    }

    public struct Category: Identifiable, Sendable {
        public var id: String { name }
        public let name: String
        public let units: [UnitInfo]
    }

    private static func u(_ symbol: String, _ name: String, _ unit: Dimension, _ aliases: [String] = []) -> UnitInfo {
        UnitInfo(symbol: symbol, name: name, unit: unit, aliases: aliases)
    }

    public static let categories: [Category] = [
        Category(name: "Longueur", units: [
            u("km", "kilomètre", UnitLength.kilometers, ["kilometre", "kilometres", "kilomètres"]),
            u("m", "mètre", UnitLength.meters, ["metre", "metres", "mètres"]),
            u("cm", "centimètre", UnitLength.centimeters, ["centimetre", "centimètres"]),
            u("mm", "millimètre", UnitLength.millimeters, ["millimètres"]),
            u("µm", "micromètre", UnitLength.micrometers, ["um"]),
            u("nm", "nanomètre", UnitLength.nanometers),
            u("mi", "mile", UnitLength.miles, ["miles", "mile"]),
            u("yd", "yard", UnitLength.yards, ["yards"]),
            u("ft", "pied", UnitLength.feet, ["pieds", "feet", "foot"]),
            u("in", "pouce", UnitLength.inches, ["pouces", "inch", "inches", "\""]),
            u("nmi", "mille marin", UnitLength.nauticalMiles, ["mille marin", "milles marins"]),
            u("al", "année-lumière", UnitLength.lightyears, ["année lumière", "années-lumière", "ly"]),
            u("ua", "unité astronomique", UnitLength.astronomicalUnits, ["au"]),
        ]),
        Category(name: "Masse", units: [
            u("t", "tonne", UnitMass.metricTons, ["tonnes"]),
            u("kg", "kilogramme", UnitMass.kilograms, ["kilo", "kilos", "kilogrammes"]),
            u("g", "gramme", UnitMass.grams, ["grammes"]),
            u("mg", "milligramme", UnitMass.milligrams, ["milligrammes"]),
            u("µg", "microgramme", UnitMass.micrograms, ["ug"]),
            u("lb", "livre", UnitMass.pounds, ["livres", "lbs", "pound", "pounds"]),
            u("oz", "once", UnitMass.ounces, ["onces", "ounce", "ounces"]),
            u("st", "stone", UnitMass.stones),
        ]),
        Category(name: "Température", units: [
            u("°C", "degré Celsius", UnitTemperature.celsius, ["c", "celsius", "degrés celsius", "degres celsius"]),
            u("°F", "degré Fahrenheit", UnitTemperature.fahrenheit, ["f", "fahrenheit"]),
            u("K", "kelvin", UnitTemperature.kelvin, ["kelvins"]),
        ]),
        Category(name: "Vitesse", units: [
            u("km/h", "kilomètre par heure", UnitSpeed(symbol: "km/h", converter: UnitConverterLinear(coefficient: 1 / 3.6)), ["kmh", "km/heure"]),
            u("m/s", "mètre par seconde", UnitSpeed.metersPerSecond, ["ms"]),
            u("mph", "mile par heure", UnitSpeed.milesPerHour),
            u("nd", "nœud", UnitSpeed.knots, ["noeud", "noeuds", "nœuds", "kn", "knots"]),
        ]),
        Category(name: "Volume", units: [
            u("m³", "mètre cube", UnitVolume.cubicMeters, ["m3"]),
            u("L", "litre", UnitVolume.liters, ["l", "litres"]),
            u("dL", "décilitre", UnitVolume.deciliters, ["dl"]),
            u("cL", "centilitre", UnitVolume.centiliters, ["cl"]),
            u("mL", "millilitre", UnitVolume.milliliters, ["ml", "millilitres"]),
            u("cm³", "centimètre cube", UnitVolume.cubicCentimeters, ["cm3", "cc"]),
            u("gal", "gallon US", UnitVolume.gallons, ["gallon", "gallons"]),
            u("tasse", "tasse US", UnitVolume.cups, ["cup", "cups", "tasses"]),
            u("fl oz", "once liquide US", UnitVolume.fluidOunces, ["floz"]),
        ]),
        Category(name: "Surface", units: [
            u("km²", "kilomètre carré", UnitArea.squareKilometers, ["km2"]),
            u("ha", "hectare", UnitArea.hectares, ["hectares"]),
            u("are", "are", UnitArea.ares, ["ares"]),
            u("m²", "mètre carré", UnitArea.squareMeters, ["m2"]),
            u("cm²", "centimètre carré", UnitArea.squareCentimeters, ["cm2"]),
            u("acre", "acre", UnitArea.acres, ["acres"]),
            u("ft²", "pied carré", UnitArea.squareFeet, ["ft2", "sqft"]),
        ]),
        Category(name: "Durée", units: [
            u("ms", "milliseconde", UnitDuration.milliseconds, ["millisecondes"]),
            u("s", "seconde", UnitDuration.seconds, ["sec", "secondes"]),
            u("min", "minute", UnitDuration.minutes, ["minutes", "mn"]),
            u("h", "heure", UnitDuration.hours, ["heures"]),
        ]),
        Category(name: "Énergie", units: [
            u("J", "joule", UnitEnergy.joules, ["joules"]),
            u("kJ", "kilojoule", UnitEnergy.kilojoules, ["kj"]),
            u("cal", "calorie", UnitEnergy.calories, ["calories"]),
            u("kcal", "kilocalorie", UnitEnergy.kilocalories, ["kilocalories"]),
            u("kWh", "kilowattheure", UnitEnergy.kilowattHours, ["kwh"]),
        ]),
        Category(name: "Pression", units: [
            u("Pa", "pascal", UnitPressure.newtonsPerMetersSquared, ["pa", "pascals"]),
            u("hPa", "hectopascal", UnitPressure.hectopascals, ["hpa"]),
            u("kPa", "kilopascal", UnitPressure.kilopascals, ["kpa"]),
            u("bar", "bar", UnitPressure.bars, ["bars"]),
            u("mbar", "millibar", UnitPressure.millibars),
            u("atm", "atmosphère", UnitPressure(symbol: "atm", converter: UnitConverterLinear(coefficient: 101_325))),
            u("psi", "livre par pouce carré", UnitPressure.poundsForcePerSquareInch),
            u("mmHg", "millimètre de mercure", UnitPressure.millimetersOfMercury, ["mmhg"]),
        ]),
        Category(name: "Données", units: [
            u("o", "octet", UnitInformationStorage.bytes, ["octets", "byte", "bytes", "B"]),
            u("Ko", "kilooctet", UnitInformationStorage.kilobytes, ["ko", "kb", "KB"]),
            u("Mo", "mégaoctet", UnitInformationStorage.megabytes, ["mo", "mb", "MB"]),
            u("Go", "gigaoctet", UnitInformationStorage.gigabytes, ["go", "gb", "GB"]),
            u("To", "téraoctet", UnitInformationStorage.terabytes, ["to", "tb", "TB"]),
            u("Gio", "gibioctet", UnitInformationStorage.gibibytes, ["gib", "GiB"]),
            u("bit", "bit", UnitInformationStorage.bits, ["bits"]),
        ]),
        Category(name: "Angle", units: [
            u("°", "degré", UnitAngle.degrees, ["deg", "degre", "degré", "degrés", "degres"]),
            u("rad", "radian", UnitAngle.radians, ["radians"]),
            u("grad", "grade", UnitAngle.gradians, ["gon", "grades"]),
            u("tr", "tour", UnitAngle.revolutions, ["tours"]),
        ]),
        Category(name: "Énergie électrique et puissance", units: [
            u("W", "watt", UnitPower.watts, ["watts"]),
            u("kW", "kilowatt", UnitPower.kilowatts, ["kw"]),
            u("ch", "cheval-vapeur", UnitPower.horsepower, ["cv", "hp", "chevaux"]),
        ]),
    ]

    public static func find(_ text: String) -> UnitInfo? {
        let t = text.trimmingCharacters(in: .whitespaces)
        let lower = t.lowercased()
        for cat in categories { for unit in cat.units where unit.symbol == t { return unit } }
        for cat in categories {
            for unit in cat.units where unit.symbol.lowercased() == lower || unit.name.lowercased() == lower
                || unit.name.lowercased() + "s" == lower || unit.aliases.contains(t) || unit.aliases.contains(lower) {
                return unit
            }
        }
        return nil
    }

    public static func category(of unit: UnitInfo) -> Category? {
        categories.first { $0.units.contains(unit) }
    }

    public static func convert(_ value: Double, from: UnitInfo, to: UnitInfo) throws -> Double {
        guard let c1 = category(of: from), let c2 = category(of: to), c1.name == c2.name else {
            throw MathError.unsupported("Impossible de convertir des \(from.name)s en \(to.name)s : grandeurs différentes.")
        }
        return Measurement(value: value, unit: from.unit).converted(to: to.unit).value
    }

    public struct ParsedConversion: Equatable, Sendable {
        public let value: Double
        public let from: UnitInfo
        public let to: UnitInfo
        public let result: Double
    }

    /// Reconnaît « 10 km en miles », « 72 °F to °C », « convertis 3 lb en kg ».
    public static func parse(_ text: String) -> ParsedConversion? {
        var t = text.trimmingCharacters(in: .whitespacesAndNewlines).trimmingCharacters(in: CharacterSet(charactersIn: "?.!"))
        for prefix in ["convertis", "convertir", "conversion de", "convert", "combien font", "combien de"] where t.lowercased().hasPrefix(prefix) {
            t = String(t.dropFirst(prefix.count)).trimmingCharacters(in: .whitespaces)
        }
        let pattern = #"^(-?[\d\s]*[.,]?\d+)\s*(.+?)\s+(?:en|in|to|vers|=|->|→)\s+(.+)$"#
        guard let re = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]),
              let m = re.firstMatch(in: t, range: NSRange(t.startIndex..., in: t)), m.numberOfRanges == 4,
              let r1 = Range(m.range(at: 1), in: t), let r2 = Range(m.range(at: 2), in: t), let r3 = Range(m.range(at: 3), in: t)
        else { return nil }
        let numberText = t[r1].replacingOccurrences(of: " ", with: "").replacingOccurrences(of: ",", with: ".")
        guard let value = Double(numberText) else { return nil }
        var fromText = String(t[r2]), toText = String(t[r3])
        for article in ["de ", "d'", "des "] where fromText.lowercased().hasPrefix(article) { fromText = String(fromText.dropFirst(article.count)) }
        for article in ["degrés ", "degres "] where toText.lowercased().hasPrefix(article) && find(toText) == nil { toText = String(toText.dropFirst(article.count)) }
        guard let from = find(fromText), let to = find(toText), let result = try? convert(value, from: from, to: to) else { return nil }
        return ParsedConversion(value: value, from: from, to: to, result: result)
    }
}
