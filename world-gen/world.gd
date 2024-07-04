# world.gd
extends Node3D 


const chunk_size = 128
const chunk_amount = 16

var noise
var chunks = {}
var unready_chunks = {}
var thread
var loading_thread = false
@export var frequency : int
@export var fractal_octaves : int
@export var fractal_lacunarity : int
#@onready var character = get_node("CameraController")
@export var character = CharacterBody3D

# Noise meathod 2
@export var noise_2 : NoiseTexture2D
@onready var noise_val = noise_2.noise



func _ready():
	
	# OLD noise meathod
	#randomize()
	#noise = OpenSimplexNoise.new()
	#noise = FastNoiseLite.new()
	#noise.noise_type = FastNoiseLite.TYPE_PERLIN
	#noise.seed = randi()
	#noise.fractal_octaves = 6
	#noise.frequency = frequency
	#noise.fractal_lacunarity = fractal_lacunarity
	#noise.fractal_type = 0 # Try different fractal types
	#print("noise:",noise)
	
	thread = Thread.new()

func _physics_process(delta):
	global_position = character.global_position.round() * Vector3(1,0,1)


func add_chunk(x,z):
	var key = str(x) + "," + str(z)
	#print("Adding chunk: ", key)  # Debugging statement
	if chunks.has(key) or unready_chunks.has(key):
		return

	if not loading_thread:
		loading_thread = true
		#load_chunk(x,z)
		#thread.start(self, "load_chunk", thread, [x, z])
		thread.start(load_chunk.bind(self, x, z))
		
		unready_chunks[key] = 1

func load_chunk(x,z): # generate chunk
	print("Loading chunk: ", x, z)  # Debugging statement
	
	# old noise method use variable:noise
	type_convert(x,TYPE_INT)
	type_convert(z,TYPE_INT)
	type_convert(chunk_size,TYPE_INT)
	
	var chunk = Chunk.new(noise_val, x*chunk_size, z*chunk_size, chunk_size) # line 54
	self.translate(Vector3(x*chunk_size, 0, z*chunk_size))
	print(self.position)
	#print_debug("chunk loaded")
	call_deferred("load_done", chunk, thread)

func load_done(chunk, thread):
	print("load done")
	add_child(chunk)
	var key = str(chunk.x / chunk_size) + "," + str(chunk.z/ chunk_size)
	chunks[key] = chunk
	unready_chunks.erase(key)
	loading_thread = false

func get_chunk(x, z):
	var key = str(x) + "," + str(z)
	if chunks.has(key):
		return chunks.get(key)

	return null

func _process(delta):
	update_chunks()
	clean_up_chunks()
	reset_chunks()

func update_chunks():
	var player_translateion = character.position # update to retrive submarine prosition
	var p_x = int(player_translateion.x) / chunk_size
	var p_z = int(player_translateion.z) / chunk_size

	#print_debug(p_x, p_z)
	
	for x in range(p_x - chunk_amount * 0.5, p_x + chunk_amount * 0.5):
		for z in range(p_z - chunk_amount * 0.5, p_z + chunk_amount * 0.5):
			add_chunk(x, z)
			var chunk = get_chunk(x,z)
			if chunk != null:
				chunk.should_remove = false

func clean_up_chunks():
	for key in chunks:
		var chunk = chunks[key]
		if chunk.should_remove:
			chunk.queue_free()
			chunks.erase(key)
			

func reset_chunks():
	for key in chunks:
		chunks[key].should_remove = true
