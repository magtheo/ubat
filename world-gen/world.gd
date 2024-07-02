# world.gd
extends Node3D

const chunk_size = 64
const chunk_amount = 16

var noise
var chunks = {}
var unready_chunks = {}
var thread
var loading_thread = false

#@onready var character = get_node("CameraController")
@export var character = CharacterBody3D

func _ready():
	randomize()
	#noise = OpenSimplexNoise.new()
	noise = FastNoiseLite.new()
	noise.seed = randi()
	noise.fractal_octaves = 6.0
	#noise.period = 80.0

	thread = Thread.new()

func add_chunk(x,z):
	var key = str(x) + "," + str(z)
	if chunks.has(key) or unready_chunks.has(key):
		return

	if not loading_thread:
		loading_thread = true
		load_chunk(x,z)
		#thread.start(self, "load_chunk", thread, [x, z])
		#thread.start(load_chunk.bind(self, thread, [x, z]))
		unready_chunks[key] = 1

func load_chunk(x,z):
	var chunk = Chunk.new(noise, x*chunk_size, z*chunk_size, chunk_size)
	self.translate(Vector3(x*chunk_size, 0, z*chunk_size))

	call_deferred("load_done", chunk, thread)


func load_done(chunk, thread):
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
