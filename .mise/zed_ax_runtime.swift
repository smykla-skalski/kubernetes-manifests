import AppKit
import ApplicationServices
import Carbon
import CoreGraphics
import Foundation

enum Mode: String {
  case frontWindow = "front-window"
  case windows
  case buttons
  case names
  case keystroke
  case type
  case keyCode = "key-code"
  case menuBarItems = "menu-bar-items"
  case menuItem = "menu-item"
  case newWindow = "new-window"
  case ensureWindow = "ensure-window"
}

struct Config {
  let mode: Mode
  let processName: String
  let appName: String
  let windowIndex: Int
  let titleContains: String
  let key: String
  let text: String
  let keyCode: CGKeyCode
  let modifiers: CGEventFlags
  let activateDelay: Double
  let appMenuName: String
  let appMenuIndex: Int
  let menuItemName: String
  let nameFilter: String
}

struct TargetApp {
  let app: NSRunningApplication
  let element: AXUIElement
}

let alphaKeyCodes: [Character: CGKeyCode] = [
  "a": 0, "s": 1, "d": 2, "f": 3, "h": 4, "g": 5, "z": 6, "x": 7, "c": 8, "v": 9,
  "b": 11, "q": 12, "w": 13, "e": 14, "r": 15, "y": 16, "t": 17, "1": 18, "2": 19,
  "3": 20, "4": 21, "6": 22, "5": 23, "=": 24, "9": 25, "7": 26, "-": 27, "8": 28,
  "0": 29, "]": 30, "o": 31, "u": 32, "[": 33, "i": 34, "p": 35, "l": 37, "j": 38,
  "'": 39, "k": 40, ";": 41, "\\": 42, ",": 43, "/": 44, "n": 45, "m": 46, ".": 47,
  "`": 50,
]

func fail(_ message: String, code: Int32 = 1) -> Never {
  fputs("\(message)\n", stderr)
  exit(code)
}

func environmentValue(_ name: String, default defaultValue: String) -> String {
  let value = ProcessInfo.processInfo.environment[name] ?? defaultValue
  return value
}

func nonNegativeInt(_ value: String, name: String) -> Int {
  guard let parsed = Int(value), parsed >= 0 else {
    fail("\(name) must be a non-negative integer, got: \(value)")
  }
  return parsed
}

func positiveInt(_ value: String, name: String) -> Int {
  guard let parsed = Int(value), parsed > 0 else {
    fail("\(name) must be a positive integer, got: \(value)")
  }
  return parsed
}

func parseMode() -> Mode {
  guard CommandLine.arguments.count >= 2 else {
    fail(
      "usage: scripts/zed_ax.swift <front-window|windows|buttons|names|keystroke|type|key-code|menu-bar-items|menu-item|new-window|ensure-window>"
    )
  }

  guard let mode = Mode(rawValue: CommandLine.arguments[1]) else {
    fail("unsupported subcommand: \(CommandLine.arguments[1])")
  }

  return mode
}

func parseModifiers(_ rawValue: String) -> CGEventFlags {
  if rawValue.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
    return []
  }

  var flags: CGEventFlags = []
  for modifier in rawValue.split(separator: ",").map({
    $0.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
  }) {
    switch modifier {
    case "":
      continue
    case "command", "cmd":
      flags.insert(.maskCommand)
    case "shift":
      flags.insert(.maskShift)
    case "option", "alt":
      flags.insert(.maskAlternate)
    case "control", "ctrl":
      flags.insert(.maskControl)
    case "fn":
      flags.insert(.maskSecondaryFn)
    default:
      fail("unsupported modifier '\(modifier)'; expected command, shift, option, control, or fn")
    }
  }

  return flags
}

func loadConfig() -> Config {
  let mode = parseMode()
  let processName = environmentValue(
    "ZED_AX_PROCESS", default: environmentValue("ZED_PROCESS", default: "zed"))
  let appName = environmentValue(
    "ZED_AX_APP_NAME", default: environmentValue("ZED_WINDOW_OWNER", default: "Zed Preview"))
  let windowIndex = positiveInt(
    environmentValue("ZED_AX_WINDOW_INDEX", default: "1"), name: "window index")
  let titleContains = ProcessInfo.processInfo.environment["ZED_AX_TITLE_CONTAINS"] ?? ""
  let key = environmentValue("ZED_AX_KEY", default: "x")
  let text = ProcessInfo.processInfo.environment["ZED_AX_TEXT"] ?? ""
  let keyCode = CGKeyCode(
    nonNegativeInt(environmentValue("ZED_AX_KEY_CODE", default: "36"), name: "key code"))
  let modifiers = parseModifiers(ProcessInfo.processInfo.environment["ZED_AX_MODIFIERS"] ?? "")
  let activateDelay = Double(environmentValue("ZED_AX_ACTIVATE_DELAY", default: "0.2")) ?? 0.2
  let appMenuName = ProcessInfo.processInfo.environment["ZED_AX_APP_MENU"] ?? "Zed"
  let appMenuIndex = positiveInt(
    environmentValue("ZED_AX_APP_MENU_INDEX", default: "2"), name: "app menu index")
  let menuItemName = ProcessInfo.processInfo.environment["ZED_AX_MENU_ITEM"] ?? "Extensions"
  let nameFilter = ProcessInfo.processInfo.environment["ZED_AX_NAME_FILTER"] ?? ""

  return Config(
    mode: mode,
    processName: processName,
    appName: appName,
    windowIndex: windowIndex,
    titleContains: titleContains,
    key: key,
    text: text,
    keyCode: keyCode,
    modifiers: modifiers,
    activateDelay: activateDelay,
    appMenuName: appMenuName,
    appMenuIndex: appMenuIndex,
    menuItemName: menuItemName,
    nameFilter: nameFilter
  )
}

func axErrorDescription(_ error: AXError) -> String {
  switch error {
  case .success: return "success"
  case .failure: return "failure"
  case .illegalArgument: return "illegalArgument"
  case .invalidUIElement: return "invalidUIElement"
  case .invalidUIElementObserver: return "invalidUIElementObserver"
  case .cannotComplete: return "cannotComplete"
  case .attributeUnsupported: return "attributeUnsupported"
  case .actionUnsupported: return "actionUnsupported"
  case .notificationUnsupported: return "notificationUnsupported"
  case .notImplemented: return "notImplemented"
  case .notificationAlreadyRegistered: return "notificationAlreadyRegistered"
  case .notificationNotRegistered: return "notificationNotRegistered"
  case .apiDisabled: return "apiDisabled"
  case .noValue: return "noValue"
  case .parameterizedAttributeUnsupported: return "parameterizedAttributeUnsupported"
  case .notEnoughPrecision: return "notEnoughPrecision"
  @unknown default: return "unknown(\(error.rawValue))"
  }
}

func copyAttribute(_ element: AXUIElement, _ attribute: String) -> Any? {
  var value: CFTypeRef?
  let error = AXUIElementCopyAttributeValue(element, attribute as CFString, &value)
  switch error {
  case .success:
    return value
  case .attributeUnsupported, .noValue, .cannotComplete:
    return nil
  default:
    fail("failed to read attribute \(attribute): \(axErrorDescription(error))")
  }
}

func elementAttribute(_ element: AXUIElement, _ attribute: String) -> AXUIElement? {
  var value: CFTypeRef?
  let error = AXUIElementCopyAttributeValue(element, attribute as CFString, &value)
  switch error {
  case .success:
    guard let value else {
      return nil
    }
    return unsafeBitCast(value, to: AXUIElement.self)
  case .attributeUnsupported, .noValue, .cannotComplete:
    return nil
  default:
    fail("failed to read attribute \(attribute): \(axErrorDescription(error))")
  }
}

func axValueAttribute(_ element: AXUIElement, _ attribute: String) -> AXValue? {
  var value: CFTypeRef?
  let error = AXUIElementCopyAttributeValue(element, attribute as CFString, &value)
  switch error {
  case .success:
    guard let value else {
      return nil
    }
    return unsafeBitCast(value, to: AXValue.self)
  case .attributeUnsupported, .noValue, .cannotComplete:
    return nil
  default:
    fail("failed to read attribute \(attribute): \(axErrorDescription(error))")
  }
}

func stringAttribute(_ element: AXUIElement, _ attribute: String) -> String? {
  guard let value = copyAttribute(element, attribute) else {
    return nil
  }
  if let stringValue = value as? String {
    let trimmed = stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
    return trimmed.isEmpty ? nil : trimmed
  }
  if let numberValue = value as? NSNumber {
    return numberValue.stringValue
  }
  return nil
}

func elementsAttribute(_ element: AXUIElement, _ attribute: String) -> [AXUIElement] {
  guard let value = copyAttribute(element, attribute) else {
    return []
  }
  return value as? [AXUIElement] ?? []
}

func pointAttribute(_ element: AXUIElement, _ attribute: String) -> CGPoint? {
  guard let value = axValueAttribute(element, attribute) else {
    return nil
  }
  guard AXValueGetType(value) == .cgPoint else {
    return nil
  }
  var point = CGPoint.zero
  guard AXValueGetValue(value, .cgPoint, &point) else {
    return nil
  }
  return point
}

func sizeAttribute(_ element: AXUIElement, _ attribute: String) -> CGSize? {
  guard let value = axValueAttribute(element, attribute) else {
    return nil
  }
  guard AXValueGetType(value) == .cgSize else {
    return nil
  }
  var size = CGSize.zero
  guard AXValueGetValue(value, .cgSize, &size) else {
    return nil
  }
  return size
}

func rectDescription(_ element: AXUIElement) -> String {
  guard let position = pointAttribute(element, kAXPositionAttribute as String),
    let size = sizeAttribute(element, kAXSizeAttribute as String)
  else {
    return "unknown"
  }

  return "\(Int(position.x)),\(Int(position.y)) \(Int(size.width))x\(Int(size.height))"
}

func attributeNames(_ element: AXUIElement) -> Set<String> {
  var names: CFArray?
  let error = AXUIElementCopyAttributeNames(element, &names)
  if error != .success {
    return []
  }
  return Set((names as? [String]) ?? [])
}

func actionNames(_ element: AXUIElement) -> Set<String> {
  var names: CFArray?
  let error = AXUIElementCopyActionNames(element, &names)
  if error != .success {
    return []
  }
  return Set((names as? [String]) ?? [])
}

func performAction(_ element: AXUIElement, _ action: String) {
  let error = AXUIElementPerformAction(element, action as CFString)
  guard error == .success else {
    fail("failed to perform \(action): \(axErrorDescription(error))")
  }
}

func executableName(for app: NSRunningApplication) -> String? {
  guard let bundleURL = app.bundleURL, let bundle = Bundle(url: bundleURL) else {
    return nil
  }
  return (bundle.object(forInfoDictionaryKey: "CFBundleExecutable") as? String)?.lowercased()
}

func appScore(_ app: NSRunningApplication, processName: String, appName: String) -> Int {
  let processName = processName.lowercased()
  let appName = appName.lowercased()
  var score = 0

  if executableName(for: app) == processName {
    score += 100
  }

  if let localizedName = app.localizedName?.lowercased() {
    if localizedName == processName || localizedName.contains(processName) {
      score += 40
    }
    if !appName.isEmpty && localizedName.contains(appName) {
      score += 30
    }
    if localizedName.contains("autofill") {
      score -= 100
    }
  }

  if let bundleName = app.bundleURL?.deletingPathExtension().lastPathComponent.lowercased() {
    if bundleName == processName || bundleName.contains(processName) {
      score += 20
    }
    if !appName.isEmpty && bundleName.contains(appName) {
      score += 15
    }
  }

  if app.activationPolicy != .prohibited {
    score += 1
  }

  return score
}

func targetApplication(config: Config) -> TargetApp {
  let runningApps = NSWorkspace.shared.runningApplications.filter { !$0.isTerminated }
  let candidates = runningApps.compactMap { app -> (Int, Int, TargetApp)? in
    let score = appScore(app, processName: config.processName, appName: config.appName)
    guard score > 0 else {
      return nil
    }

    let element = AXUIElementCreateApplication(app.processIdentifier)
    let windowCount = windowCandidates(for: element).count
    return (score, windowCount, TargetApp(app: app, element: element))
  }

  guard
    let match = candidates.max(by: { lhs, rhs in
      if lhs.0 != rhs.0 {
        return lhs.0 < rhs.0
      }
      return lhs.1 < rhs.1
    })
  else {
    fail("no running '\(config.processName)' process matching '\(config.appName)' was found")
  }

  return match.2
}

func windowRole(_ element: AXUIElement) -> String {
  stringAttribute(element, kAXRoleAttribute as String) ?? "AXUnknown"
}

func windowSubrole(_ element: AXUIElement) -> String {
  stringAttribute(element, kAXSubroleAttribute as String) ?? "AXUnknown"
}

func windowTitle(_ element: AXUIElement) -> String {
  stringAttribute(element, kAXTitleAttribute as String) ?? ""
}

func windowCandidates(for appElement: AXUIElement) -> [AXUIElement] {
  var candidates = elementsAttribute(appElement, kAXWindowsAttribute as String)
    .filter { windowRole($0) == (kAXWindowRole as String) }

  if let focused = elementAttribute(appElement, kAXFocusedWindowAttribute as String),
    !candidates.contains(where: { CFEqual($0, focused) })
  {
    candidates.insert(focused, at: 0)
  }

  if let main = elementAttribute(appElement, kAXMainWindowAttribute as String),
    !candidates.contains(where: { CFEqual($0, main) })
  {
    candidates.append(main)
  }

  return candidates
}

func selectedWindow(for target: TargetApp, config: Config) -> AXUIElement {
  if config.mode == .frontWindow {
    if let focused = elementAttribute(target.element, kAXFocusedWindowAttribute as String) {
      return focused
    }
    if let main = elementAttribute(target.element, kAXMainWindowAttribute as String) {
      return main
    }
  }

  let appWindows = windowCandidates(for: target.element)
  guard !appWindows.isEmpty else {
    fail("process '\(config.processName)' has no Accessibility windows")
  }

  if !config.titleContains.isEmpty,
    let match = appWindows.first(where: {
      windowTitle($0).localizedCaseInsensitiveContains(config.titleContains)
    })
  {
    return match
  }

  let requestedIndex = config.windowIndex - 1
  guard requestedIndex < appWindows.count else {
    fail("window index \(config.windowIndex) is out of range for process '\(config.processName)'")
  }

  return appWindows[requestedIndex]
}

func childElements(of element: AXUIElement) -> [AXUIElement] {
  let navigationChildren = elementsAttribute(element, "AXChildrenInNavigationOrder")
  if !navigationChildren.isEmpty {
    return navigationChildren
  }
  return elementsAttribute(element, kAXChildrenAttribute as String)
}

func descendants(of root: AXUIElement, maxDepth: Int = 16) -> [AXUIElement] {
  var result = [AXUIElement]()
  var queue: [(AXUIElement, Int)] = [(root, 0)]

  while !queue.isEmpty {
    let (current, depth) = queue.removeFirst()
    result.append(current)

    if depth >= maxDepth {
      continue
    }

    for child in childElements(of: current) {
      queue.append((child, depth + 1))
    }
  }

  return result
}

func labels(for element: AXUIElement) -> [String] {
  let candidateAttributes = [
    "AXIdentifier",
    kAXTitleAttribute as String,
    kAXDescriptionAttribute as String,
    kAXRoleDescriptionAttribute as String,
    kAXHelpAttribute as String,
    kAXValueAttribute as String,
  ]

  var values = [String]()
  for attribute in candidateAttributes {
    if let value = stringAttribute(element, attribute), !value.isEmpty {
      values.append(value)
    }
  }

  var uniqueValues = [String]()
  var seen = Set<String>()
  for value in values where seen.insert(value).inserted {
    uniqueValues.append(value)
  }

  return uniqueValues
}

func formattedElement(_ element: AXUIElement) -> String? {
  let labels = labels(for: element)
  guard !labels.isEmpty else {
    return nil
  }
  let role = windowRole(element)
  let subrole = stringAttribute(element, kAXSubroleAttribute as String)
  let labelText = labels.joined(separator: " | ")
  if let subrole, !subrole.isEmpty {
    return "\(role)/\(subrole)\t\(labelText)"
  }
  return "\(role)\t\(labelText)"
}

func selectedMenuBarItem(target: TargetApp, menuName: String, menuIndex: Int) -> AXUIElement {
  guard let menuBar = elementAttribute(target.element, kAXMenuBarAttribute as String) else {
    fail(
      "process '\(target.app.localizedName ?? config.processName)' does not expose an Accessibility menu bar"
    )
  }

  let items = childElements(of: menuBar)
  guard !items.isEmpty else {
    fail(
      "process '\(target.app.localizedName ?? config.processName)' has an empty Accessibility menu bar"
    )
  }

  if !menuName.isEmpty,
    let match = items.first(where: {
      windowTitle($0) == menuName || windowTitle($0).localizedCaseInsensitiveContains(menuName)
    })
  {
    return match
  }

  let requestedIndex = menuIndex - 1
  guard requestedIndex < items.count else {
    fail("menu bar index \(menuIndex) is out of range")
  }

  return items[requestedIndex]
}

func openMenu(_ menuBarItem: AXUIElement) -> AXUIElement {
  if let menu = elementAttribute(menuBarItem, "AXMenu") {
    return menu
  }

  if actionNames(menuBarItem).contains(kAXPressAction as String) {
    performAction(menuBarItem, kAXPressAction as String)
    Thread.sleep(forTimeInterval: 0.1)
  }

  if let menu = elementAttribute(menuBarItem, "AXMenu") {
    return menu
  }

  if let menu = childElements(of: menuBarItem).first(where: {
    windowRole($0) == (kAXMenuRole as String)
  }) {
    return menu
  }

  fail("failed to resolve menu for menu bar item '\(windowTitle(menuBarItem))'")
}

func findDescendant(in root: AXUIElement, titleEquals title: String, role: String? = nil)
  -> AXUIElement?
{
  descendants(of: root).first { element in
    if let role, windowRole(element) != role {
      return false
    }
    return labels(for: element).contains(where: {
      $0 == title || $0.localizedCaseInsensitiveContains(title)
    })
  }
}

func clickMenuItem(
  target: TargetApp, menuBarTitle: String, menuBarIndex: Int, menuItemTitle: String
) {
  let menuBarItem = selectedMenuBarItem(
    target: target, menuName: menuBarTitle, menuIndex: menuBarIndex)
  let menu = openMenu(menuBarItem)
  guard
    let menuItem = findDescendant(
      in: menu, titleEquals: menuItemTitle, role: kAXMenuItemRole as String)
  else {
    fail(
      "failed to find menu item '\(menuItemTitle)' in menu '\(menuBarTitle.isEmpty ? String(menuBarIndex) : menuBarTitle)'"
    )
  }
  performAction(menuItem, kAXPressAction as String)
}

func clickNewWindow(target: TargetApp) {
  clickMenuItem(target: target, menuBarTitle: "File", menuBarIndex: 4, menuItemTitle: "New Window")
}

func ensureWindow(target: TargetApp) {
  let existingWindows = windowCandidates(for: target.element)
  if !existingWindows.isEmpty {
    print(
      "process '\(config.processName)' already has \(existingWindows.count) Accessibility window(s) across Spaces"
    )
    return
  }

  clickNewWindow(target: target)

  for _ in 0..<20 {
    Thread.sleep(forTimeInterval: 0.1)
    let currentWindows = windowCandidates(for: target.element)
    if !currentWindows.isEmpty {
      print(
        "process '\(config.processName)' now has \(currentWindows.count) Accessibility window(s) across Spaces"
      )
      return
    }
  }

  fail("failed to create a new Accessibility-visible window for process '\(config.processName)'")
}

func keyCode(for key: String) -> CGKeyCode {
  guard key.count == 1, let character = key.lowercased().first,
    let keyCode = alphaKeyCodes[character]
  else {
    fail("unsupported key '\(key)'; only single US-layout printable characters are supported")
  }
  return keyCode
}

func postKeyCode(_ keyCode: CGKeyCode, flags: CGEventFlags, pid: pid_t) {
  guard let keyDown = CGEvent(keyboardEventSource: nil, virtualKey: keyCode, keyDown: true),
    let keyUp = CGEvent(keyboardEventSource: nil, virtualKey: keyCode, keyDown: false)
  else {
    fail("failed to create keyboard events")
  }

  keyDown.flags = flags
  keyUp.flags = flags
  keyDown.postToPid(pid)
  keyUp.postToPid(pid)
}

func postUnicodeText(_ text: String, pid: pid_t) {
  for scalar in text.utf16 {
    var buffer = [scalar]
    guard let keyDown = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: true),
      let keyUp = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: false)
    else {
      fail("failed to create unicode keyboard events")
    }

    keyDown.keyboardSetUnicodeString(stringLength: 1, unicodeString: &buffer)
    keyUp.keyboardSetUnicodeString(stringLength: 1, unicodeString: &buffer)
    keyDown.postToPid(pid)
    keyUp.postToPid(pid)
  }
}

let config = loadConfig()
let target = targetApplication(config: config)

switch config.mode {
case .frontWindow:
  let window = selectedWindow(for: target, config: config)
  let title = windowTitle(window)
  print(title.isEmpty ? "[untitled]" : title)
case .windows:
  let appWindows = windowCandidates(for: target.element)
  guard !appWindows.isEmpty else {
    fail("process '\(config.processName)' has no Accessibility windows")
  }

  for (index, window) in appWindows.enumerated() {
    let title = windowTitle(window).isEmpty ? "[untitled]" : windowTitle(window)
    print(
      "index=\(index + 1) role=\(windowRole(window)) subrole=\(windowSubrole(window)) "
        + "bounds=\(rectDescription(window)) title=\(title)"
    )
  }
case .buttons:
  let window = selectedWindow(for: target, config: config)
  let lines = descendants(of: window)
    .filter { windowRole($0) == (kAXButtonRole as String) }
    .compactMap { formattedElement($0) }

  for line in lines {
    print(line)
  }
case .names:
  let window = selectedWindow(for: target, config: config)
  let lines = descendants(of: window)
    .compactMap { formattedElement($0) }
    .filter { config.nameFilter.isEmpty || $0.localizedCaseInsensitiveContains(config.nameFilter) }

  for line in lines {
    print(line)
  }
case .menuBarItems:
  guard let menuBar = elementAttribute(target.element, kAXMenuBarAttribute as String) else {
    fail("process '\(config.processName)' does not expose an Accessibility menu bar")
  }

  for item in childElements(of: menuBar) {
    let title = windowTitle(item)
    print(title.isEmpty ? "[untitled]" : title)
  }
case .menuItem:
  clickMenuItem(
    target: target, menuBarTitle: config.appMenuName, menuBarIndex: config.appMenuIndex,
    menuItemTitle: config.menuItemName)
case .newWindow:
  clickNewWindow(target: target)
case .ensureWindow:
  ensureWindow(target: target)
case .keystroke:
  Thread.sleep(forTimeInterval: max(0, config.activateDelay))
  postKeyCode(keyCode(for: config.key), flags: config.modifiers, pid: target.app.processIdentifier)
case .type:
  Thread.sleep(forTimeInterval: max(0, config.activateDelay))
  postUnicodeText(config.text, pid: target.app.processIdentifier)
case .keyCode:
  Thread.sleep(forTimeInterval: max(0, config.activateDelay))
  postKeyCode(config.keyCode, flags: config.modifiers, pid: target.app.processIdentifier)
}
