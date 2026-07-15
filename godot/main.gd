extends Node2D

const SNAPSHOT_PATH := "res://.tmp/godot-world.json"
const REQUEST_PATH := "res://.tmp/godot-view-request.json"
const PANEL_WIDTH := 282.0
const TOP_BAR_HEIGHT := 46.0
const MAP_BACKGROUND := Color("141814")
const PANEL_BACKGROUND := Color("23241f")
const PANEL_BORDER := Color("54513f")
const TEXT := Color("ebe5d3")
const MUTED_TEXT := Color("c6bea5")

var map_center := Vector2.ZERO
var home_center := Vector2.ZERO
var tile_size := 32.0
var panning := false
var dragged := false
var selected_nation_id = -1
var show_history := false
var show_borders := true
var world_data: Dictionary = {}
var tile_owners: Dictionary = {}
var nation_lookup: Dictionary = {}
var next_snapshot_read := 0.0
var clock := 0.0
var status_text := "Waiting for the Rust simulation snapshot."

var summary_label: Label
var inspector_label: RichTextLabel
var status_label: Label
var history_button: Button
var grass_texture: Texture2D
var shore_texture: Texture2D
var tree_texture: Texture2D
var rock_texture: Texture2D
var wall_texture: Texture2D
var worker_texture: Texture2D
var soldier_texture: Texture2D
var boar_texture: Texture2D
var fish_texture: Texture2D
var building_textures: Dictionary = {}

func _ready() -> void:
	grass_texture = load("res://assets/MiniWorldSprites/Ground/Grass.png")
	shore_texture = load("res://assets/MiniWorldSprites/Ground/Shore.png")
	tree_texture = load("res://assets/MiniWorldSprites/Nature/Trees.png")
	rock_texture = load("res://assets/MiniWorldSprites/Nature/Rocks.png")
	wall_texture = load("res://assets/MiniWorldSprites/Buildings/Enemy/Orc/Walls.png")
	worker_texture = load("res://assets/MiniWorldSprites/Characters/Workers/CyanWorker/FarmerCyan.png")
	soldier_texture = load("res://assets/MiniWorldSprites/Characters/Soldiers/Melee/CyanMelee/SwordsmanCyan.png")
	boar_texture = load("res://assets/MiniWorldSprites/Animals/Boar.png")
	fish_texture = load("res://assets/MiniWorldSprites/Animals/MarineAnimals.png")
	building_textures = {
		"keep": [load("res://assets/MiniWorldSprites/Buildings/Wood/Keep.png"), load("res://assets/MiniWorldSprites/Buildings/Cyan/CyanKeep.png"), load("res://assets/MiniWorldSprites/Buildings/Lime/LimeKeep.png"), load("res://assets/MiniWorldSprites/Buildings/Purple/PurpleKeep.png"), load("res://assets/MiniWorldSprites/Buildings/Red/RedKeep.png")],
		"house": [load("res://assets/MiniWorldSprites/Buildings/Wood/Houses.png"), load("res://assets/MiniWorldSprites/Buildings/Cyan/CyanHouses.png"), load("res://assets/MiniWorldSprites/Buildings/Lime/LimeHouses.png"), load("res://assets/MiniWorldSprites/Buildings/Purple/PurpleHouses.png"), load("res://assets/MiniWorldSprites/Buildings/Red/RedHouses.png")],
		"farm": [load("res://assets/MiniWorldSprites/Buildings/Enemy/Orc/Farms.png")],
		"wheatfield": [load("res://assets/MiniWorldSprites/Nature/Wheatfield.png")],
		"hut": [load("res://assets/MiniWorldSprites/Buildings/Wood/Huts.png"), load("res://assets/MiniWorldSprites/Buildings/Cyan/CyanHuts.png"), load("res://assets/MiniWorldSprites/Buildings/Lime/LimeHuts.png"), load("res://assets/MiniWorldSprites/Buildings/Purple/PurpleHuts.png"), load("res://assets/MiniWorldSprites/Buildings/Red/RedHuts.png")],
	}
	build_ui()
	get_viewport().size_changed.connect(queue_redraw)
	write_view_request()

func _process(delta: float) -> void:
	clock += delta
	sync_snapshot()
	queue_redraw()

func sync_snapshot() -> void:
	if clock < next_snapshot_read:
		return
	next_snapshot_read = clock + 0.25
	write_view_request()
	var file := FileAccess.open(SNAPSHOT_PATH, FileAccess.READ)
	if file == null:
		status_text = "Waiting for the Rust simulation snapshot."
		refresh_ui()
		return
	var decoded: Variant = JSON.parse_string(file.get_as_text())
	if typeof(decoded) == TYPE_DICTIONARY:
		apply_world_view(decoded)
		status_text = "Rust simulation connected"
		refresh_ui()

func write_view_request() -> void:
	var file := FileAccess.open(REQUEST_PATH, FileAccess.WRITE)
	if file != null:
		file.store_string(JSON.stringify({"type": "view", "center": {"x": roundi(map_center.x), "y": roundi(map_center.y)}, "radius": view_radius()}))

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_WHEEL_UP or event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			var before := screen_to_world(event.position)
			var factor := 1.16 if event.button_index == MOUSE_BUTTON_WHEEL_UP else 1.0 / 1.16
			tile_size = clampf(tile_size * factor, 28.0, 48.0)
			map_center = before - (event.position - map_screen_center()) / tile_size
			next_snapshot_read = clock
		elif event.button_index == MOUSE_BUTTON_LEFT:
			panning = event.pressed
			dragged = false
			if not event.pressed and not dragged:
				select_at(event.position)
	elif event is InputEventMouseMotion and panning:
		if event.relative.length() > 0.0:
			dragged = true
			map_center -= event.relative / tile_size
			next_snapshot_read = clock

func _draw() -> void:
	var viewport := get_viewport_rect().size
	draw_rect(Rect2(Vector2.ZERO, viewport), MAP_BACKGROUND)
	if world_data.is_empty():
		return
	for tile in world_data.get("tiles", []):
		var rect := tile_rect(Vector2(tile.x, tile.y))
		if rect.intersects(Rect2(Vector2(0, TOP_BAR_HEIGHT), Vector2(viewport.x - PANEL_WIDTH, viewport.y))):
			draw_tile(rect, tile)
	if show_borders:
		draw_borders()
	for path in world_data.get("paths", []):
		var points: Array[Vector2] = []
		for tile in path.get("tiles", []):
			points.append(tile_screen_position(Vector2(tile.x, tile.y)))
		for index in range(1, points.size()):
			var route_color := Color("cca55b") if path.trade else Color("473623")
			draw_line(points[index - 1], points[index], route_color, maxf(1.0, tile_size * 0.16))
	for building in world_data.get("buildings", []):
		draw_building(building)
	for animal in world_data.get("wildlife", []):
		draw_wildlife(animal)
	for worker in world_data.get("workers", []):
		draw_worker(worker)
	for nation in world_data.get("nations", []):
		var capital := tile_screen_position(Vector2(nation.capital.x, nation.capital.y))
		if Rect2(Vector2.ZERO, Vector2(viewport.x - PANEL_WIDTH, viewport.y)).has_point(capital):
			draw_circle(capital, maxf(3.0, tile_size * 0.26), nation_color(nation.id))
			if int(nation.id) == selected_nation_id:
				draw_arc(capital, maxf(7.0, tile_size * 0.48), 0.0, TAU, 20, Color("f5d67a"), 1.5)
	draw_conflicts()

func draw_tile(rect: Rect2, tile: Dictionary) -> void:
	var texture := grass_texture
	var frame := 2
	if tile.terrain in ["water", "ocean", "lake", "river"]:
		frame = 0
	elif tile.terrain in ["beach", "desert"]:
		texture = shore_texture
		frame = 0
	elif tile.terrain == "prairie":
		frame = 1
	if texture != null:
		draw_texture_rect_region(texture, rect.grow(0.5), Rect2(Vector2(frame * 16, 0), Vector2(16, 16)))
	else:
		draw_rect(rect, terrain_color(tile.terrain))
	var owner = tile.get("owner", null)
	if owner != null:
		draw_rect(rect.grow(-maxf(1.0, tile_size * 0.12)), nation_color(owner, 0.22), false, maxf(1.0, tile_size * 0.08))
	if tile_size >= 10.0:
		match tile.get("feature", ""):
			"tree":
				draw_sprite(tree_texture, rect, 16)
			"rock":
				draw_sprite(rock_texture, rect, 16)
		if tile.wall:
			draw_sprite(wall_texture, rect, 16)

func draw_building(building: Dictionary) -> void:
	var rect := tile_rect(Vector2(building.location.x, building.location.y)).grow(-tile_size * 0.08)
	var kind: String = building.kind
	match building.kind:
		"keep":
			draw_sprite(building_texture(building.nation, kind), rect.grow(tile_size * 0.22), 32)
		_:
			draw_sprite(building_texture(building.nation, kind), rect, 16)

func draw_worker(worker: Dictionary) -> void:
	if tile_size < 8.0:
		return
	var point := tile_screen_position(Vector2(worker.location.x, worker.location.y))
	var rect := Rect2(point - Vector2.ONE * tile_size * 0.35, Vector2.ONE * tile_size * 0.7)
	var texture := soldier_texture if worker.role == "hunter" else worker_texture
	draw_sprite_frame(texture, rect, 16, int(clock * 8.0 + worker.id) % 5)
	var ring := Color("daa65c") if worker.role == "hunter" else Color("7eb45c") if worker.role == "lumberjack" else Color("97b1cc")
	draw_arc(point, tile_size * 0.32, 0.0, TAU, 12, ring, 1.0)

func draw_wildlife(animal: Dictionary) -> void:
	if tile_size < 8.0:
		return
	var point := tile_screen_position(Vector2(animal.location.x, animal.location.y))
	var rect := Rect2(point - Vector2.ONE * tile_size * 0.35, Vector2.ONE * tile_size * 0.7)
	if animal.kind == "fish":
		draw_sprite_frame(fish_texture, rect, 16, int(animal.heading) % 4 * 5 + 2)
	else:
		draw_sprite_frame(boar_texture, rect, 16, int(animal.heading) % 4 * 4 + int(clock * 8.0 + animal.id) % 4)

func draw_borders() -> void:
	for tile in world_data.get("tiles", []):
		var owner = tile.get("owner", null)
		if owner == null:
			continue
		var position := Vector2i(int(tile.x), int(tile.y))
		var rect := tile_rect(Vector2(position))
		var color := nation_color(owner)
		for edge in [[Vector2i.UP, rect.position, rect.position + Vector2.RIGHT * rect.size.x], [Vector2i.RIGHT, rect.position + Vector2.RIGHT * rect.size.x, rect.end], [Vector2i.DOWN, rect.position + Vector2.DOWN * rect.size.y, rect.end], [Vector2i.LEFT, rect.position, rect.position + Vector2.DOWN * rect.size.y]]:
			if tile_owners.get(position + edge[0], null) != owner:
				draw_line(edge[1], edge[2], Color("1d2319"), maxf(2.5, tile_size * 0.19))
				draw_line(edge[1], edge[2], color, maxf(1.0, tile_size * 0.10))

func draw_conflicts() -> void:
	for conflict in world_data.get("conflicts", []):
		var attacker: Dictionary = nation_lookup.get(int(conflict.attacker), {})
		var defender: Dictionary = nation_lookup.get(int(conflict.defender), {})
		if attacker.is_empty() or defender.is_empty():
			continue
		var point := (tile_screen_position(Vector2(attacker.capital.x, attacker.capital.y)) + tile_screen_position(Vector2(defender.capital.x, defender.capital.y))) * 0.5
		var size := maxf(4.0, tile_size * 0.30)
		draw_line(point - Vector2.ONE * size, point + Vector2.ONE * size, Color("bb5344"), 2.0)
		draw_line(point + Vector2(-size, size), point + Vector2(size, -size), Color("bb5344"), 2.0)

func building_texture(nation_id: int, kind: String) -> Texture2D:
	var textures: Array = building_textures.get(kind, building_textures.hut)
	return textures[posmod(nation_id, textures.size())]

func draw_sprite(texture: Texture2D, rect: Rect2, cell_size: int) -> void:
	draw_sprite_frame(texture, rect, cell_size, 0)

func draw_sprite_frame(texture: Texture2D, rect: Rect2, cell_size: int, frame: int) -> void:
	if texture != null:
		var columns: int = max(1, texture.get_width() / cell_size)
		var rows: int = max(1, texture.get_height() / cell_size)
		var index := posmod(frame, columns * rows)
		draw_texture_rect_region(texture, rect, Rect2(Vector2(index % columns, index / columns) * cell_size, Vector2.ONE * cell_size))

func select_at(position: Vector2) -> void:
	if position.x >= get_viewport_rect().size.x - PANEL_WIDTH:
		return
	var tile_pos := screen_to_world(position).floor()
	for tile in world_data.get("tiles", []):
		if int(tile.x) == int(tile_pos.x) and int(tile.y) == int(tile_pos.y):
			selected_nation_id = int(tile.get("owner", -1))
			show_history = false
			refresh_ui()
			return

func build_ui() -> void:
	var layer := CanvasLayer.new()
	add_child(layer)
	var top := panel()
	top.set_anchors_preset(Control.PRESET_TOP_WIDE)
	top.offset_bottom = TOP_BAR_HEIGHT
	layer.add_child(top)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 12)
	top.add_child(row)
	summary_label = Label.new()
	summary_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	summary_label.add_theme_color_override("font_color", TEXT)
	row.add_child(summary_label)
	var home := Button.new()
	home.text = "Home"
	home.pressed.connect(func(): map_center = home_center; next_snapshot_read = clock)
	row.add_child(home)
	history_button = Button.new()
	history_button.text = "History"
	history_button.pressed.connect(func(): show_history = not show_history; refresh_ui())
	row.add_child(history_button)
	var borders := Button.new()
	borders.text = "Borders"
	borders.toggle_mode = true
	borders.button_pressed = true
	borders.toggled.connect(func(pressed: bool): show_borders = pressed)
	row.add_child(borders)
	var side := panel()
	side.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	side.offset_left = -PANEL_WIDTH
	side.offset_top = TOP_BAR_HEIGHT
	side.offset_right = 0
	side.offset_bottom = 0
	layer.add_child(side)
	var content := VBoxContainer.new()
	content.add_theme_constant_override("separation", 10)
	side.add_child(content)
	inspector_label = RichTextLabel.new()
	inspector_label.bbcode_enabled = true
	inspector_label.fit_content = false
	inspector_label.scroll_active = true
	inspector_label.size_flags_vertical = Control.SIZE_EXPAND_FILL
	inspector_label.add_theme_color_override("default_color", TEXT)
	content.add_child(inspector_label)
	status_label = Label.new()
	status_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	status_label.add_theme_color_override("font_color", MUTED_TEXT)
	content.add_child(status_label)
	refresh_ui()

func panel() -> PanelContainer:
	var result := PanelContainer.new()
	var style := StyleBoxFlat.new()
	style.bg_color = PANEL_BACKGROUND
	style.border_color = PANEL_BORDER
	style.set_border_width_all(1)
	style.content_margin_left = 14
	style.content_margin_top = 10
	style.content_margin_right = 14
	style.content_margin_bottom = 10
	result.add_theme_stylebox_override("panel", style)
	return result

func refresh_ui() -> void:
	if summary_label == null:
		return
	if world_data.is_empty():
		summary_label.text = "ThreadNations"
		inspector_label.text = "[b]Map viewer[/b]\n\nThe Godot viewer is waiting for the Rust simulation."
		status_label.text = status_text
		return
	summary_label.text = "Year %s  |  %s nations  |  %s conflicts  |  %s activity" % [world_data.year, world_data.nations.size(), world_data.conflicts.size(), world_data.activity_total]
	history_button.text = "Nation" if show_history else "History"
	status_label.text = status_text + "\nDrag to pan. Mouse wheel zooms. Click a nation to inspect it."
	if show_history:
		var lines: PackedStringArray = ["[b]History[/b]"]
		for event in world_data.get("history", []):
			lines.append("\n%s" % event.summary)
		inspector_label.text = "".join(lines)
		return
	var nation := selected_nation()
	if nation.is_empty():
		inspector_label.text = "[b]Nation[/b]\n\nSelect a nation on the map for its full report."
		return
	inspector_label.text = "[b]%s[/b]\n%s\n\nPopulation  %s\nTerritory  %s tiles\nMilitary  %s\nStability  %s\n\nFood  %s\nMaterials  %s\nWealth  %s\nResearch  %s" % [nation.name, nation.ruler, nation.population, nation.territory, nation.military, nation.stability, nation.food, nation.materials, nation.wealth, nation.research]

func selected_nation() -> Dictionary:
	for nation in world_data.get("nations", []):
		if int(nation.id) == selected_nation_id:
			return nation
	return {}

func apply_world_view(view: Dictionary) -> void:
	var first_view := world_data.is_empty()
	world_data = view
	tile_owners.clear()
	nation_lookup.clear()
	for tile in world_data.get("tiles", []):
		tile_owners[Vector2i(int(tile.x), int(tile.y))] = tile.get("owner", null)
	for nation in world_data.get("nations", []):
		nation_lookup[int(nation.id)] = nation
	if first_view and not world_data.nations.is_empty():
		var capital = world_data.nations[0].capital
		map_center = Vector2(capital.x, capital.y)
		home_center = map_center

func map_screen_center() -> Vector2:
	var size := get_viewport_rect().size
	return Vector2((size.x - PANEL_WIDTH) * 0.5, (size.y + TOP_BAR_HEIGHT) * 0.5)

func screen_to_world(position: Vector2) -> Vector2:
	return map_center + (position - map_screen_center()) / tile_size

func tile_screen_position(position: Vector2) -> Vector2:
	return map_screen_center() + (position - map_center) * tile_size

func tile_rect(position: Vector2) -> Rect2:
	return Rect2(tile_screen_position(position) - Vector2.ONE * tile_size * 0.5, Vector2.ONE * tile_size)

func view_radius() -> int:
	var size := get_viewport_rect().size
	# ponytail: keep snapshots interactive; add chunk/delta caching before supporting wider low-zoom views.
	return clampi(ceili(maxf(size.x - PANEL_WIDTH, size.y) / tile_size * 0.65), 16, 24)

func terrain_color(terrain: String) -> Color:
	var colors := {"water": Color("2c4e4b"), "ocean": Color("4dbab8"), "lake": Color("4dbab8"), "river": Color("4dbab8"), "beach": Color("ead59d"), "plains": Color("536539"), "grassland": Color("6a9d46"), "prairie": Color("b9d557"), "forest": Color("2e4c31"), "hills": Color("675b41"), "mountains": Color("5b554e"), "desert": Color("907c51"), "wetlands": Color("47624c")}
	return colors.get(terrain, Color("6a9d46"))

func nation_color(id: int, alpha := 1.0) -> Color:
	var colors := [Color("c66f46"), Color("b19741"), Color("859d52"), Color("966985"), Color("be8054"), Color("709789")]
	var color: Color = colors[posmod(id, colors.size())]
	color.a = alpha
	return color
