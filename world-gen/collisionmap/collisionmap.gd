extends CollisionShape3D

@export var character : Node3D

@export var template_mesh : PlaneMesh
@onready var faces = template_mesh.get_faces()
@onready var snap = Vector3.ONE * template_mesh.size.x/2



func _ready():
	#print("collision mesh faces: ", typeof(faces))
	#update_shape() # uncommended during creation of secondWorld
	pass

#func _physics_process(delta):
	#var player_rounded_position = character.global_position.snapped(snap) * Vector3(1,0,1)
	#if not global_position == player_rounded_position:
		#global_position = player_rounded_position
		#update_shape()

#func update_shape():
	##print(faces.size())
	#for i in faces.size():
		#var global_vert = faces[i] + global_position
		#faces[i].y = await Heightmap.get_height(global_vert.x, global_vert.z)
	#shape.set_faces(faces)
