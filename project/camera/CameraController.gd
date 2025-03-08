extends CharacterBody3D

@onready var rotator: Node3D = $rotator
@onready var camera_3d: Camera3D = $rotator/Camera3D

@onready var omni_light_3d: OmniLight3D = $rotator/OmniLight3D
@onready var spot_light_3d: SpotLight3D = $rotator/SpotLight3D

var lights_on: bool = true
var SPEED = 0.08
var MOUSE_SENSITIVITY = 0.05

const FORWARD = Vector3(0, 1, 0)
const BACK = Vector3(0, -1, 0)
const LEFT = Vector3(-1, 0, 0)
const RIGHT = Vector3(1, 0, 0)
const UPWARD = Vector3(0,0,1)
const DOWNWARD = Vector3(0,0,-1)

func _ready():
	await get_tree().physics_frame

func _unhandled_input(event):
	if event is InputEventMouseButton:
		Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
	elif event.is_action_pressed("ui_cancel"):
		Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)

func _input(event):
	if event is InputEventMouseMotion:
		var mouse_motion_x = event.relative.x * MOUSE_SENSITIVITY
		var mouse_motion_y = event.relative.y * MOUSE_SENSITIVITY

		rotator.rotation_degrees.y -= mouse_motion_x
		rotator.rotation_degrees.x = clamp(rotator.rotation_degrees.x - mouse_motion_y, -90, 90)

	if event is InputEventKey and event.pressed and event.keycode == KEY_L:
		lights_on = !lights_on
		omni_light_3d.visible = lights_on
		spot_light_3d.visible = lights_on
		print("Lights toggled:", lights_on)  # Debugging output


func _physics_process(_delta):
	# Movement
	if Input.is_action_pressed("move_forward"):
		translate(DOWNWARD * SPEED)
	if Input.is_action_pressed("move_backward"):
		translate(UPWARD * SPEED)
	if Input.is_action_pressed("move_left"):
		translate(LEFT * SPEED)
	if Input.is_action_pressed("move_right"):
		translate(RIGHT * SPEED)
	if Input.is_action_pressed("move_up"):
		translate(FORWARD * SPEED)
	if Input.is_action_pressed("move_down"):
		translate(BACK * SPEED)
