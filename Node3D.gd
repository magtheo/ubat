extends Node3D

@onready var child = $child

# Called when the node enters the scene tree for the first time.
func _ready():
	var child_instance = child.new()


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass
