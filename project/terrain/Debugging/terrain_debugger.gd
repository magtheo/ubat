extends Control

var _is_initialized: bool = false # Flag to track if references are set

# --- Internal References (Set by _attempt_initialization) ---
var terrain_bridge: Node = null
@export var player: Node = null

# --- UI Node References (CORRECTED PATHS) ---
@onready var lbl_player_world_pos = $PanelContainer/MarginContainer/VBoxContainer/lblPlayerWorldPos
@onready var lbl_player_chunk_pos = $PanelContainer/MarginContainer/VBoxContainer/lblPlayerChunkPos
@onready var lbl_chunk_state = $PanelContainer/MarginContainer/VBoxContainer/lblChunkState
@onready var lbl_height = $PanelContainer/MarginContainer/VBoxContainer/lblHeight
@onready var lbl_biome = $PanelContainer/MarginContainer/VBoxContainer/lblBiome
# Add labels for section, weights etc. as needed

# Note: Check if "VisOptionsContainer" is the correct name in your scene tree
@onready var chk_vis_normal = $PanelContainer/MarginContainer/VBoxContainer/VisOptionsContainer/chkVisNormal
@onready var chk_vis_height = $PanelContainer/MarginContainer/VBoxContainer/VisOptionsContainer/chkVisHeight
@onready var chk_vis_biome_id = $PanelContainer/MarginContainer/VBoxContainer/VisOptionsContainer/chkVisBiomeID
# Use ButtonGroup resource for RadioButtons if preferred

var _current_player_pos := Vector3.ZERO
var _current_player_chunk := Vector2i.ZERO
var _last_updated_chunk := Vector2i(INF, INF) # Force initial update

const UPDATE_INTERVAL = 0.2 # Update labels every 0.2 seconds
var _update_timer = 0.0

# --- Rest of the script (_ready, _process, etc.) remains the same ---
# --- Make sure the node names (like "lblPlayerWorldPos", "VisOptionsContainer", "chkVisNormal") ---
# --- exactly match the names you gave them in the Godot editor scene tree ---

func _ready():
	print("TerrainDebugger: Ready. Waiting for initialization...")
	
	# --- Validate Player Export ---
	if not is_instance_valid(player):
		push_warning("TerrainDebugger: Player node NOT assigned via @export variable in the editor!")
		# Initialization might fail later if player is required

	# Connect UI signals (these are internal to this scene, safe to connect)
	if is_instance_valid(chk_vis_normal):
		if not chk_vis_normal.is_connected("toggled", Callable(self, "_on_vis_mode_toggled")):
			chk_vis_normal.toggled.connect(_on_vis_mode_toggled.bind(0, chk_vis_normal))
	else: push_error("chkVisNormal node not found at expected path!")

	if is_instance_valid(chk_vis_height):
		if not chk_vis_height.is_connected("toggled", Callable(self, "_on_vis_mode_toggled")):
			chk_vis_height.toggled.connect(_on_vis_mode_toggled.bind(1, chk_vis_height))
	else: push_error("chkVisHeight node not found at expected path!")

	if is_instance_valid(chk_vis_biome_id):
		if not chk_vis_biome_id.is_connected("toggled", Callable(self, "_on_vis_mode_toggled")):
			chk_vis_biome_id.toggled.connect(_on_vis_mode_toggled.bind(2, chk_vis_biome_id))
	else: push_error("chkVisBiomeID node not found at expected path!")

	# Set initial UI state (visual only, doesn't affect controller yet)
	if is_instance_valid(chk_vis_normal): chk_vis_normal.button_pressed = true
	if is_instance_valid(chk_vis_height): chk_vis_height.button_pressed = false
	if is_instance_valid(chk_vis_biome_id): chk_vis_biome_id.button_pressed = false

	# Don't update labels yet, wait for initialization
	if is_instance_valid(lbl_player_world_pos): lbl_player_world_pos.text = "World Pos: Initializing..."
	if is_instance_valid(lbl_player_chunk_pos): lbl_player_chunk_pos.text = "Chunk Pos: Initializing..."
	if is_instance_valid(lbl_chunk_state): lbl_chunk_state.text = "Chunk State: Initializing..."
	if is_instance_valid(lbl_height): lbl_height.text = "Height: Initializing..."
	if is_instance_valid(lbl_biome): lbl_biome.text = "Biome: Initializing..."

func _attempt_initialization():
	# Stage 1: Check BridgeManager (using direct access)
	var bridge_manager = BridgeManager
	if not is_instance_valid(bridge_manager):
		print("TerrainDebugger: BridgeManager instance INVALID.") # Simplified log
		return

	# Stage 2: Check if BridgeManager finished its initialization
	var flag_is_set = bridge_manager.all_bridges_initialized
	var potential_terrain_bridge = bridge_manager.get_terrain_bridge()
	var bridge_is_valid = is_instance_valid(potential_terrain_bridge)

	print("TerrainDebugger: Frame %d - Checking init state: Flag=%s, BridgeValid=%s" % [Engine.get_process_frames(), flag_is_set, bridge_is_valid])

	if not (flag_is_set and bridge_is_valid):
		return # Still waiting

	# --- Stage 3: Check Player Node (now uses the @export var) ---
	if not is_instance_valid(self.player): # Check the exported variable
		print("TerrainDebugger: Bridges ready, waiting for exported Player node instance...")
		# Ensure player is assigned in the editor!
		return # Still waiting for player

	# --- FINAL SUCCESS ---
	print("TerrainDebugger: ALL Initialization checks PASSED this frame (BridgeManager, TerrainBridge, Player).")

	# Assign references (only terrain_bridge needed now)
	self.terrain_bridge = potential_terrain_bridge
	# self.player is already assigned via @export

	_is_initialized = true # Set the flag!

	# --- Connect player signal ---
	var signal_name = "player_chunk_changed"
	# Check player validity again just before connecting
	if is_instance_valid(self.player):
		if self.player.has_signal(signal_name):
			var callable = Callable(self, "_on_player_chunk_changed")
			if not self.player.is_connected(signal_name, callable):
				var err = self.player.connect(signal_name, callable)
				if err != OK:
					push_error("TerrainDebugger: Failed to connect player signal '%s', error: %s" % [signal_name, err])
				else:
					print("TerrainDebugger: Successfully connected '%s' signal to _on_player_chunk_changed" % signal_name)
		else:
			push_warning("Exported Player node does not have '%s' signal." % signal_name)
	else:
		push_warning("Exported Player node is invalid when trying to connect signal.")


	# Set initial debug mode
	var cc = terrain_bridge.get_chunk_controller()
	if is_instance_valid(cc):
		cc.set_debug_visualization_mode(0)
	else:
		push_warning("TerrainDebugger: ChunkController not valid during initialization.")

	# Force initial update (deferred)
	call_deferred("_force_initial_player_update")



# --- Helper for initial update ---
func _force_initial_player_update():
	print("TerrainDebugger: Running _force_initial_player_update...") # ADDED
	if not _is_initialized:
		print("TerrainDebugger:  ...but not initialized yet, aborting.") # ADDED
		return
	# Use the exported player variable
	if is_instance_valid(self.player):
		var initial_pos = Vector3.ZERO
		if self.player.has_method("get_global_position"):
			initial_pos = self.player.get_global_position()
		elif self.player.has_method("get_position"):
			initial_pos = self.player.get_position()
		print("TerrainDebugger:   Got initial player pos: ", initial_pos) # ADDED

		var signal_name = "player_chunk_changed"
		if self.player.has_signal(signal_name):
			var cs = 32.0 # TODO: Get chunk size properly if it changes
			var initial_chunk_x = floori(initial_pos.x / cs)
			var initial_chunk_z = floori(initial_pos.z / cs)
			print("TerrainDebugger:   Calling _on_player_chunk_changed with initial chunk: %d, %d" % [initial_chunk_x, initial_chunk_z]) # ADDED
			_on_player_chunk_changed(initial_chunk_x, initial_chunk_z) # This calls _update_debug_labels
	else:
		print("TerrainDebugger:   Exported Player instance invalid in _force_initial_player_update.") # ADDED

	print("TerrainDebugger:   Calling _update_debug_labels directly after signal/pos handling.") # ADDED
	_update_debug_labels()



func _process(delta):
	if not _is_initialized:
		_attempt_initialization()
		return

	# --- Main Logic ---
	# Poll world pos only if signal is missing
	if is_instance_valid(self.player):
		var signal_name = "player_chunk_changed"
		if not self.player.has_signal(signal_name):
			# ... (polling logic remains same, using self.player) ...
			var world_pos = Vector3.ZERO
			if player.has_method("get_global_position"):
				world_pos = player.get_global_position()
			elif player.has_method("get_position"):
				world_pos = player.get_position()
			if world_pos.distance_squared_to(_current_player_pos) > 0.01:
				_current_player_pos = world_pos
				var cc = terrain_bridge.get_chunk_controller()
				if is_instance_valid(cc): _current_player_chunk = cc.get_player_chunk_coords()

	# Update timer logic
	_update_timer += delta
	if _update_timer >= UPDATE_INTERVAL:
		_update_timer = 0.0
		print("TerrainDebugger: Timer triggered, calling _update_debug_labels.") # ADDED
		_update_debug_labels()


func _on_player_chunk_changed(chunk_x: int, chunk_z: int):
	print("TerrainDebugger: _on_player_chunk_changed called with %d, %d" % [chunk_x, chunk_z]) # ADDED
	if not _is_initialized: return

	_current_player_chunk = Vector2i(chunk_x, chunk_z)

	# Use the exported player variable
	if is_instance_valid(self.player):
		if self.player.has_method("get_global_position"):
			_current_player_pos = self.player.get_global_position()
		elif self.player.has_method("get_position"):
			_current_player_pos = self.player.get_position()
	else:
		print("TerrainDebugger: Player invalid in _on_player_chunk_changed") # ADDED

	# Update labels immediately on chunk change
	_update_debug_labels()
	_last_updated_chunk = _current_player_chunk


# --- _update_debug_labels() (Needs update for Chunk State) ---
func _update_debug_labels():
	print("TerrainDebugger: _update_debug_labels called.") # ADDED
	# Use the flag for the primary check
	if not _is_initialized:
		print("TerrainDebugger:   Not initialized, returning.") # ADDED
		return

	# Check essential references
	if not is_instance_valid(terrain_bridge):
		print("TerrainDebugger:   terrain_bridge invalid, returning.") # ADDED
		# ... (set error text) ...
		if is_instance_valid(lbl_player_world_pos): lbl_player_world_pos.text = "World Pos: ERR (No Bridge)"
		return
	if not is_instance_valid(self.player): # Also check player here? Might be overkill if checked before calling
		print("TerrainDebugger:   player instance invalid, returning.") # ADDED
		if is_instance_valid(lbl_player_world_pos): lbl_player_world_pos.text = "World Pos: ERR (No Player)"
		return


	# Get managers via bridge
	var cm = terrain_bridge.get_chunk_manager()
	var bm = terrain_bridge.get_biome_manager()

	# Check if managers are valid
	if not is_instance_valid(cm) or not is_instance_valid(bm): # Assuming BiomeManager is also required
		print("TerrainDebugger:   ChunkManager or BiomeManager invalid via bridge, returning. CM valid: %s, BM valid: %s" % [is_instance_valid(cm), is_instance_valid(bm)]) # ADDED
		# Set error text
		if is_instance_valid(lbl_player_world_pos): lbl_player_world_pos.text = "World Pos: ERR (No CM/BM)"
		if is_instance_valid(lbl_chunk_state): lbl_chunk_state.text = "Chunk State: ERR (No CM/BM)"
		if is_instance_valid(lbl_height): lbl_height.text = "Height: ERR (No CM/BM)"
		if is_instance_valid(lbl_biome): lbl_biome.text = "Biome: ERR (No CM/BM)"
		return

	# --- If we reach here, all references should be valid ---
	print("TerrainDebugger:   Managers valid, proceeding to update labels.") # ADDED

	# --- Update UI elements ---
	# ... (rest of UI update logic remains the same, using self.player, cm, bm) ...
	if is_instance_valid(lbl_player_world_pos):
		lbl_player_world_pos.text = "World Pos: %.1f, %.1f, %.1f" % [_current_player_pos.x, _current_player_pos.y, _current_player_pos.z]
	if is_instance_valid(lbl_player_chunk_pos):
		lbl_player_chunk_pos.text = "Chunk Pos: %d, %d" % [_current_player_chunk.x, _current_player_chunk.y]

	if is_instance_valid(lbl_chunk_state):
		var state_code = cm.get_chunk_state_at(_current_player_chunk.x, _current_player_chunk.y)
		var state_text = "N/A"
		match state_code:
			-1: state_text = "Untracked"
			0: state_text = "Unknown"
			1: state_text = "Loading"
			2: state_text = "Generating"
			3: state_text = "Ready"
		lbl_chunk_state.text = "Chunk State: %s (%d)" % [state_text, state_code]

	var terrain_data: Dictionary = cm.get_terrain_data_at(_current_player_pos.x, _current_player_pos.z)

	if is_instance_valid(lbl_height):
		if terrain_data.has("height") and terrain_data["height"] != null:
			lbl_height.text = "Height: %.2f" % terrain_data["height"]
		else:
			lbl_height.text = "Height: N/A"

	if is_instance_valid(lbl_biome):
		var biome_text = "Biome: N/A"
		if terrain_data.has("primary_biome_id") and terrain_data["primary_biome_id"] != null:
			var biome_id = terrain_data["primary_biome_id"]
			if is_instance_valid(bm) and bm.has_method("get_biome_name"):
				var biome_name = bm.get_biome_name(biome_id)
				biome_text = "Biome: %s (%d)" % [biome_name, biome_id]
			else:
				biome_text = "Biome: ID %d (No BM)" % biome_id
		lbl_biome.text = biome_text



func _on_player_position_updated(world_pos: Vector3):
	#print("TerrainDebugger: player position updated: ", world_pos)
	if not is_instance_valid(terrain_bridge): return # Need bridge

	_current_player_pos = world_pos
	var cc = terrain_bridge.get_chunk_controller() # Use bridge getter
	if is_instance_valid(cc):
		_current_player_chunk = cc.get_player_chunk_coords()
	else:
		_current_player_chunk = Vector2i.ZERO # Reset or keep old?


func _on_vis_mode_toggled(pressed: bool, mode: int, checkbox: CheckBox):
	# Check bridge validity first
	if not is_instance_valid(terrain_bridge): return
	# Check UI elements validity
	if not is_instance_valid(chk_vis_normal) or \
	   not is_instance_valid(chk_vis_height) or \
	   not is_instance_valid(chk_vis_biome_id): return


	var cc = terrain_bridge.get_chunk_controller() # Get controller via bridge
	if not is_instance_valid(cc):
		push_error("Cannot set debug mode: ChunkController not available via TerrainBridge")
		checkbox.button_pressed = !pressed # Revert toggle
		# Ensure at least one is checked
		if not pressed and not chk_vis_height.button_pressed and not chk_vis_biome_id.button_pressed:
			chk_vis_normal.button_pressed = true
		return

	var target_mode = 0
	# --- Radio-button logic for checkboxes ---
	if pressed:
		target_mode = mode
		if checkbox != chk_vis_normal: chk_vis_normal.button_pressed = false
		if checkbox != chk_vis_height: chk_vis_height.button_pressed = false
		if checkbox != chk_vis_biome_id: chk_vis_biome_id.button_pressed = false
		checkbox.button_pressed = true # Ensure pressed state
	else:
		# Prevent unchecking the last active debug mode, revert to normal if all are off
		var any_debug_active = chk_vis_height.button_pressed or chk_vis_biome_id.button_pressed
		if not any_debug_active: # If height/biomeID are off...
			chk_vis_normal.button_pressed = true # ...force normal on
			target_mode = 0
		else: # Another debug mode is still active
			# Prevent unchecking if it would leave no box checked
			if not chk_vis_normal.button_pressed and not chk_vis_height.button_pressed and not chk_vis_biome_id.button_pressed:
				checkbox.button_pressed = true # Force it back on
			return # Don't change the mode

	print("Setting debug visualization mode to: ", target_mode)
	cc.set_debug_visualization_mode(target_mode) # Call controller via bridge
