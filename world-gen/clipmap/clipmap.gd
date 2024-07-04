extends Node3D

@export var player : Node3D
@onready var sprite_2d = $Sprite2D

func _physics_process(delta):
	global_position = player.global_position.round() * Vector3(1,0,1)

