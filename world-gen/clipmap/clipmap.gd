extends Node3D

@export var player : Node3D
@onready var height_rect = $"../HeightRect"


func _physics_process(delta):
	# Code beneeth will make the terrain "generate infinatly" using the same noise
	#global_position = player.global_position.round() * Vector3(1,0,1)
	pass
