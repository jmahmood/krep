/// Core domain types for the Cardio Microdose system.
///
/// This module defines the fundamental types used throughout the system:
/// - Movements and their properties
/// - Metrics (reps, bands, etc.)
/// - Microdose definitions and sessions
/// - User state and progression tracking
/// - Strength signal integration
///
/// Direct port from Rust `cardio_core/src/types.rs`

import Foundation

// MARK: - Movement Types

/// Type of movement/exercise
public enum MovementKind: String, Codable, Equatable {
    case kettlebellSwing = "kettlebell_swing"
    case burpee
    case pullup
    case mobilityDrill = "mobility_drill"
}

/// Burpee variation styles
public enum BurpeeStyle: String, Codable, Equatable {
    case fourCount = "four_count"
    case sixCount = "six_count"
    case sixCountTwoPump = "six_count_two_pump"
    case seal

    /// Get the next progression level
    public func next() -> BurpeeStyle? {
        switch self {
        case .fourCount: return .sixCount
        case .sixCount: return .sixCountTwoPump
        case .sixCountTwoPump: return .seal
        case .seal: return nil // Max level
        }
    }
}

/// Specification for resistance bands
public enum BandSpec: Codable, Equatable {
    case none
    case namedColour(String)

    enum CodingKeys: String, CodingKey {
        case type
        case colour
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let type = try container.decode(String.self, forKey: .type)

        switch type {
        case "none":
            self = .none
        case "named_colour":
            let colour = try container.decode(String.self, forKey: .colour)
            self = .namedColour(colour)
        default:
            throw DecodingError.dataCorrupted(
                DecodingError.Context(codingPath: decoder.codingPath,
                                    debugDescription: "Unknown band spec type: \(type)")
            )
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .none:
            try container.encode("none", forKey: .type)
        case .namedColour(let colour):
            try container.encode("named_colour", forKey: .type)
            try container.encode(colour, forKey: .colour)
        }
    }
}

/// Style variations for movements
public enum MovementStyle: Codable, Equatable {
    case none
    case burpee(BurpeeStyle)
    case band(BandSpec)

    enum CodingKeys: String, CodingKey {
        case type
        case style
        case spec
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let type = try container.decode(String.self, forKey: .type)

        switch type {
        case "none":
            self = .none
        case "burpee":
            let style = try container.decode(BurpeeStyle.self, forKey: .style)
            self = .burpee(style)
        case "band":
            let spec = try container.decode(BandSpec.self, forKey: .spec)
            self = .band(spec)
        default:
            throw DecodingError.dataCorrupted(
                DecodingError.Context(codingPath: decoder.codingPath,
                                    debugDescription: "Unknown movement style type: \(type)")
            )
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .none:
            try container.encode("none", forKey: .type)
        case .burpee(let style):
            try container.encode("burpee", forKey: .type)
            try container.encode(style, forKey: .style)
        case .band(let spec):
            try container.encode("band", forKey: .type)
            try container.encode(spec, forKey: .spec)
        }
    }
}

/// A movement definition (e.g., "Kettlebell Swing")
public struct Movement: Codable {
    public let id: String
    public let name: String
    public let kind: MovementKind
    public let defaultStyle: MovementStyle
    public let tags: [String]
    public let referenceUrl: String?

    enum CodingKeys: String, CodingKey {
        case id, name, kind, tags
        case defaultStyle = "default_style"
        case referenceUrl = "reference_url"
    }

    public init(id: String, name: String, kind: MovementKind, defaultStyle: MovementStyle,
                tags: [String], referenceUrl: String? = nil) {
        self.id = id
        self.name = name
        self.kind = kind
        self.defaultStyle = defaultStyle
        self.tags = tags
        self.referenceUrl = referenceUrl
    }
}

// MARK: - Metric Types

/// Metric specification with type-safe variants
public enum MetricSpec: Codable {
    case reps(key: String, defaultValue: Int, min: Int, max: Int, step: Int, progressable: Bool)
    case band(key: String, defaultValue: String, progressable: Bool)

    enum CodingKeys: String, CodingKey {
        case type, key
        case defaultValue = "default"
        case min, max, step, progressable
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let type = try container.decode(String.self, forKey: .type)

        switch type {
        case "reps":
            let key = try container.decode(String.self, forKey: .key)
            let defaultValue = try container.decode(Int.self, forKey: .defaultValue)
            let min = try container.decode(Int.self, forKey: .min)
            let max = try container.decode(Int.self, forKey: .max)
            let step = try container.decode(Int.self, forKey: .step)
            let progressable = try container.decode(Bool.self, forKey: .progressable)
            self = .reps(key: key, defaultValue: defaultValue, min: min, max: max,
                        step: step, progressable: progressable)
        case "band":
            let key = try container.decode(String.self, forKey: .key)
            let defaultValue = try container.decode(String.self, forKey: .defaultValue)
            let progressable = try container.decode(Bool.self, forKey: .progressable)
            self = .band(key: key, defaultValue: defaultValue, progressable: progressable)
        default:
            throw DecodingError.dataCorrupted(
                DecodingError.Context(codingPath: decoder.codingPath,
                                    debugDescription: "Unknown metric spec type: \(type)")
            )
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .reps(let key, let defaultValue, let min, let max, let step, let progressable):
            try container.encode("reps", forKey: .type)
            try container.encode(key, forKey: .key)
            try container.encode(defaultValue, forKey: .defaultValue)
            try container.encode(min, forKey: .min)
            try container.encode(max, forKey: .max)
            try container.encode(step, forKey: .step)
            try container.encode(progressable, forKey: .progressable)
        case .band(let key, let defaultValue, let progressable):
            try container.encode("band", forKey: .type)
            try container.encode(key, forKey: .key)
            try container.encode(defaultValue, forKey: .defaultValue)
            try container.encode(progressable, forKey: .progressable)
        }
    }
}

// MARK: - Microdose Block and Definition Types

/// A single work block within a microdose (e.g., one EMOM interval)
public struct MicrodoseBlock: Codable {
    public let movementId: String
    public let movementStyle: MovementStyle
    public let durationHintSeconds: UInt32
    public let metrics: [MetricSpec]

    enum CodingKeys: String, CodingKey {
        case movementId = "movement_id"
        case movementStyle = "movement_style"
        case durationHintSeconds = "duration_hint_seconds"
        case metrics
    }

    public init(movementId: String, movementStyle: MovementStyle,
                durationHintSeconds: UInt32, metrics: [MetricSpec]) {
        self.movementId = movementId
        self.movementStyle = movementStyle
        self.durationHintSeconds = durationHintSeconds
        self.metrics = metrics
    }
}

/// Category of microdose workout
public enum MicrodoseCategory: String, Codable, Equatable, Hashable {
    case vo2
    case gtg
    case mobility

    /// Get the next category in round-robin rotation
    public func next() -> MicrodoseCategory {
        switch self {
        case .vo2: return .gtg
        case .gtg: return .mobility
        case .mobility: return .vo2
        }
    }
}

/// A complete microdose workout definition
public struct MicrodoseDefinition: Codable {
    public let id: String
    public let name: String
    public let category: MicrodoseCategory
    public let suggestedDurationSeconds: UInt32
    public let gtgFriendly: Bool
    public let blocks: [MicrodoseBlock]
    public let referenceUrl: String?

    enum CodingKeys: String, CodingKey {
        case id, name, category, blocks
        case suggestedDurationSeconds = "suggested_duration_seconds"
        case gtgFriendly = "gtg_friendly"
        case referenceUrl = "reference_url"
    }

    public init(id: String, name: String, category: MicrodoseCategory,
                suggestedDurationSeconds: UInt32, gtgFriendly: Bool,
                blocks: [MicrodoseBlock], referenceUrl: String? = nil) {
        self.id = id
        self.name = name
        self.category = category
        self.suggestedDurationSeconds = suggestedDurationSeconds
        self.gtgFriendly = gtgFriendly
        self.blocks = blocks
        self.referenceUrl = referenceUrl
    }
}

// MARK: - Session and State Types

/// A recorded microdose session
public struct MicrodoseSession: Codable {
    public let id: UUID
    public let definitionId: String
    public let performedAt: Date
    public let startedAt: Date?
    public let completedAt: Date?
    public let actualDurationSeconds: UInt32?
    public let metricsRealized: [MetricSpec]
    public let perceivedRpe: UInt8?
    public let avgHR: UInt8?
    public let maxHR: UInt8?

    enum CodingKeys: String, CodingKey {
        case id
        case definitionId = "definition_id"
        case performedAt = "performed_at"
        case startedAt = "started_at"
        case completedAt = "completed_at"
        case actualDurationSeconds = "actual_duration_seconds"
        case metricsRealized = "metrics_realized"
        case perceivedRpe = "perceived_rpe"
        case avgHR = "avg_hr"
        case maxHR = "max_hr"
    }

    public init(id: UUID = UUID(), definitionId: String, performedAt: Date = Date(),
                startedAt: Date? = nil, completedAt: Date? = nil,
                actualDurationSeconds: UInt32? = nil, metricsRealized: [MetricSpec] = [],
                perceivedRpe: UInt8? = nil, avgHR: UInt8? = nil, maxHR: UInt8? = nil) {
        self.id = id
        self.definitionId = definitionId
        self.performedAt = performedAt
        self.startedAt = startedAt
        self.completedAt = completedAt
        self.actualDurationSeconds = actualDurationSeconds
        self.metricsRealized = metricsRealized
        self.perceivedRpe = perceivedRpe
        self.avgHR = avgHR
        self.maxHR = maxHR
    }
}

/// Type-level distinction between real sessions and skipped prescriptions
///
/// This ensures that skipped sessions (used only for influencing the prescription
/// engine) can never accidentally reach persistence layers.
public enum SessionKind {
    case real(MicrodoseSession)
    case shownButSkipped(definitionId: String, shownAt: Date)

    /// Get the definition ID for this session (works for both Real and ShownButSkipped)
    public var definitionId: String {
        switch self {
        case .real(let session):
            return session.definitionId
        case .shownButSkipped(let definitionId, _):
            return definitionId
        }
    }

    /// Get the timestamp when this session/prescription occurred
    public var timestamp: Date {
        switch self {
        case .real(let session):
            return session.performedAt
        case .shownButSkipped(_, let shownAt):
            return shownAt
        }
    }

    /// Check if this is a Real session (returns nil for ShownButSkipped)
    public func asReal() -> MicrodoseSession? {
        if case .real(let session) = self {
            return session
        }
        return nil
    }
}

/// Progression state for a specific microdose definition
public struct ProgressionState: Codable {
    public var reps: Int
    public var style: MovementStyle
    public var level: UInt32
    public var lastUpgraded: Date?

    enum CodingKeys: String, CodingKey {
        case reps, style, level
        case lastUpgraded = "last_upgraded"
    }

    public init(reps: Int, style: MovementStyle, level: UInt32 = 0, lastUpgraded: Date? = nil) {
        self.reps = reps
        self.style = style
        self.level = level
        self.lastUpgraded = lastUpgraded
    }
}

/// User's persistent state across sessions
public struct UserMicrodoseState: Codable {
    public var progressions: [String: ProgressionState]
    public var lastMobilityDefId: String?

    enum CodingKeys: String, CodingKey {
        case progressions
        case lastMobilityDefId = "last_mobility_def_id"
    }

    public init(progressions: [String: ProgressionState] = [:], lastMobilityDefId: String? = nil) {
        self.progressions = progressions
        self.lastMobilityDefId = lastMobilityDefId
    }
}

/// Type of strength training session
public enum StrengthSessionType: Codable, Equatable {
    case lower
    case upper
    case full
    case other(String)

    enum CodingKeys: String, CodingKey {
        case type, value
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let type = try container.decode(String.self, forKey: .type)

        switch type {
        case "lower": self = .lower
        case "upper": self = .upper
        case "full": self = .full
        case "other":
            let value = try container.decode(String.self, forKey: .value)
            self = .other(value)
        default:
            throw DecodingError.dataCorrupted(
                DecodingError.Context(codingPath: decoder.codingPath,
                                    debugDescription: "Unknown strength session type: \(type)")
            )
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .lower: try container.encode("lower", forKey: .type)
        case .upper: try container.encode("upper", forKey: .type)
        case .full: try container.encode("full", forKey: .type)
        case .other(let value):
            try container.encode("other", forKey: .type)
            try container.encode(value, forKey: .value)
        }
    }
}

/// External strength training signal (from another system)
public struct ExternalStrengthSignal: Codable {
    public let lastSessionAt: Date
    public let sessionType: StrengthSessionType

    enum CodingKeys: String, CodingKey {
        case lastSessionAt = "last_session_at"
        case sessionType = "session_type"
    }

    public init(lastSessionAt: Date, sessionType: StrengthSessionType) {
        self.lastSessionAt = lastSessionAt
        self.sessionType = sessionType
    }
}

/// Runtime context for prescription engine
public struct UserContext {
    public let now: Date
    public var userState: UserMicrodoseState
    public let recentSessions: [SessionKind]
    public let externalStrength: ExternalStrengthSignal?
    public let equipmentAvailable: [String]

    public init(now: Date = Date(), userState: UserMicrodoseState,
                recentSessions: [SessionKind] = [], externalStrength: ExternalStrengthSignal? = nil,
                equipmentAvailable: [String] = []) {
        self.now = now
        self.userState = userState
        self.recentSessions = recentSessions
        self.externalStrength = externalStrength
        self.equipmentAvailable = equipmentAvailable
    }
}

// MARK: - Catalog Type

/// The complete catalog of movements and microdose definitions
public struct Catalog {
    public let movements: [String: Movement]
    public let microdoses: [String: MicrodoseDefinition]

    public init(movements: [String: Movement], microdoses: [String: MicrodoseDefinition]) {
        self.movements = movements
        self.microdoses = microdoses
    }
}

// MARK: - Prescribed Microdose Result

/// The result of calling the prescription engine
public struct PrescribedMicrodose {
    public let definition: MicrodoseDefinition
    public let reps: Int
    public let style: MovementStyle

    public init(definition: MicrodoseDefinition, reps: Int, style: MovementStyle) {
        self.definition = definition
        self.reps = reps
        self.style = style
    }
}
