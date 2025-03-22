extends CharacterBody3D

@onready var rotator: Node3D = $rotator
@onready var camera_3d: Camera3D = $rotator/Camera3D

var SPEED = 0.1
var MOUSE_SENSITIVITY = 0.1

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
		var mouse_motion = Vector3()
		mouse_motion.x = event.relative.x * 0.1
		mouse_motion.y = event.relative.y * 0.1
		# You would need to get the z component of the mouse motion here, depending on your setup

		# Constrain yaw and pitch
		var target_yaw = clamp(rotator.rotation.y + mouse_motion.x, -180, 180)
		rotator.rotate_y(deg_to_rad(target_yaw))

		# Keep the camera's up direction aligned with the world's up direction
		var target_pitch = deg_to_rad(fmod(rotator.rotation.x + mouse_motion.y, PI))
		if abs(target_pitch) < 0.05:
			target_pitch = 0
		elif target_pitch > 0:
			target_pitch = clamp(target_pitch, 0, deg_to_rad(PI/2))
		elif target_pitch < 0:
			target_pitch = clamp(target_pitch, -deg_to_rad(PI/2), 0)
		rotator.rotate_x(deg_to_rad(target_pitch))

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
