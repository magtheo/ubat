extends Node3D

@onready var door = $"."
@onready var handle = $Door/Torus_001
@onready var interactObj = $InteractionV2
@onready var marker = $Door/Torus_001/Marker3D

# Called when the node enters the scene tree for the first time.
func _ready():
	interactObj.assign_and_connect("hei", Callable(self,"_door_interact"))


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass
func _door_interact(): 
	while (true): 
		
		
		
	
