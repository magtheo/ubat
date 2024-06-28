extends CharacterBody3D

@onready var camera = $rotationHelper/PhantomCamera3D
@onready var rotation_helper = $rotationHelper

var SPEED = 100
var MOUSE_SENSITIVITY = 10

const FORWARD = Vector3(0, 1, 0)
const BACK = Vector3(0, -1, 0)
const LEFT = Vector3(-1, 0, 0)
const RIGHT = Vector3(1, 0, 0)
const UPWARD = Vector3(0,0,1)
const DOWNWARD = Vector3(0,0,-1)

func _ready():
	await get_tree().physics_frame


func _physics_process(delta):
	# Movement
	if Input.is_action_pressed("move_forward"):
		translate(FORWARD * SPEED)
	if Input.is_action_pressed("move_backward"):
		translate(BACK * SPEED)
	if Input.is_action_pressed("move_left"):
		translate(LEFT * SPEED)
	if Input.is_action_pressed("move_right"):
		translate(RIGHT * SPEED)
	if Input.is_action_pressed("move_up"):
		translate(UPWARD * SPEED)
	if Input.is_action_pressed("move_down"):
		translate(DOWNWARD * SPEED)

	# Rotation
	var camera_rotation = camera.rotation
	camera_rotation.x += (get_viewport().get_mouse_position().x - camera_rotation.x) / MOUSE_SENSITIVITY
	camera_rotation.y += (get_viewport().get_mouse_position().y - camera_rotation.y) / MOUSE_SENSITIVITY
	camera.set_rotation(camera_rotation)

