import CoreGraphics
import Foundation

enum Mode: String {
  case count
  case list
}

struct WindowSummary {
  let ownerName: String
  let windowName: String
  let layer: Int
  let ownerPID: Int
  let isOnscreen: Int
  let bounds: CGRect?
}

func parseArguments() -> (mode: Mode, ownerSubstring: String) {
  var mode: Mode = .list
  var ownerSubstring = ProcessInfo.processInfo.environment["ZED_WINDOW_OWNER"] ?? "Zed Preview"
  var index = 1
  let arguments = CommandLine.arguments

  while index < arguments.count {
    let argument = arguments[index]
    switch argument {
    case "count":
      mode = .count
    case "list":
      mode = .list
    case "--owner-substring":
      index += 1
      guard index < arguments.count else {
        fputs("missing value for --owner-substring\n", stderr)
        exit(2)
      }
      ownerSubstring = arguments[index]
    default:
      fputs("unsupported argument: \(argument)\n", stderr)
      exit(2)
    }
    index += 1
  }

  return (mode, ownerSubstring)
}

func windowSummaries(ownerSubstring: String) -> [WindowSummary] {
  let windowInfo =
    CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID) as? [[String: Any]] ?? []
  let lowercasedOwnerSubstring = ownerSubstring.lowercased()

  return windowInfo.compactMap { window in
    let ownerName = (window[kCGWindowOwnerName as String] as? String) ?? ""
    guard ownerName.lowercased().contains(lowercasedOwnerSubstring) else {
      return nil
    }

    let windowName = (window[kCGWindowName as String] as? String) ?? ""
    let layer = (window[kCGWindowLayer as String] as? Int) ?? -1
    let ownerPID = (window[kCGWindowOwnerPID as String] as? Int) ?? -1
    let isOnscreen = (window[kCGWindowIsOnscreen as String] as? Int) ?? -1
    let boundsDictionary = window[kCGWindowBounds as String] as? NSDictionary
    let bounds = boundsDictionary.flatMap { CGRect(dictionaryRepresentation: $0) }

    return WindowSummary(
      ownerName: ownerName,
      windowName: windowName,
      layer: layer,
      ownerPID: ownerPID,
      isOnscreen: isOnscreen,
      bounds: bounds
    )
  }
}

func format(bounds: CGRect?) -> String {
  guard let bounds else {
    return "unknown"
  }

  return
    "\(Int(bounds.origin.x)),\(Int(bounds.origin.y)) \(Int(bounds.size.width))x\(Int(bounds.size.height))"
}

let arguments = parseArguments()
let summaries = windowSummaries(ownerSubstring: arguments.ownerSubstring)

switch arguments.mode {
case .count:
  print(summaries.count)
case .list:
  for summary in summaries {
    let name = summary.windowName.isEmpty ? "[untitled]" : summary.windowName
    print(
      "owner=\(summary.ownerName) pid=\(summary.ownerPID) layer=\(summary.layer) "
        + "onscreen=\(summary.isOnscreen) bounds=\(format(bounds: summary.bounds)) name=\(name)"
    )
  }
}
